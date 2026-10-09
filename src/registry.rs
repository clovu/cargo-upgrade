//! Release lookup through the crates.io sparse index.

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use futures::{StreamExt, stream};
use reqwest::{Client, StatusCode};
use semver::Version;
use serde::Deserialize;

/// Published, non-yanked versions of each crate, or why they could not be fetched.
pub type Releases<'a> = BTreeMap<&'a str, Result<Vec<Version>>>;

pub struct CratesIo {
    client: Client,
}

impl CratesIo {
    const INDEX: &str = "https://index.crates.io";
    const USER_AGENT: &str = concat!(
        env!("CARGO_PKG_NAME"),
        "/",
        env!("CARGO_PKG_VERSION"),
        " (",
        env!("CARGO_PKG_REPOSITORY"),
        ")"
    );
    /// Lookups in flight at once; enough to hide latency without flooding the connection.
    const CONCURRENCY: usize = 8;
    /// Retry transient failures up to three times: about 250ms, 500ms, then 1s apart.
    const BACKOFF: ExponentialBuilder = ExponentialBuilder::new()
        .with_min_delay(Duration::from_millis(250))
        .with_max_times(3)
        .with_jitter();

    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(Self::USER_AGENT)
            .build()
            .expect("HTTP client configuration is valid");
        Self { client }
    }

    /// Looks up every crate, a few at a time.
    pub async fn releases<'a>(&self, crates: impl IntoIterator<Item = &'a str>) -> Releases<'a> {
        // A crate may appear in several sections; look each up once.
        let crates: BTreeSet<_> = crates.into_iter().collect();
        stream::iter(crates)
            .map(|name| async move { (name, self.versions(name).await) })
            .buffer_unordered(Self::CONCURRENCY)
            .collect()
            .await
    }

    async fn versions(&self, name: &str) -> Result<Vec<Version>> {
        let url = format!("{}/{}", Self::INDEX, index_path(name));
        let context = || format!("failed to look up `{name}` on crates.io {url}");
        let body = (|| self.get(&url))
            .retry(Self::BACKOFF)
            .when(is_transient)
            .await
            .with_context(context)?;
        let releases: Vec<Release> = body
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .with_context(context)?;
        Ok(releases
            .into_iter()
            .filter(|r| !r.yanked)
            .map(|r| r.vers)
            .collect())
    }

    async fn get(&self, url: &str) -> reqwest::Result<String> {
        self.client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
    }
}

/// Whether a failed request is worth repeating, as with cargo's own `net.retry`:
/// dropped connections and timeouts are, a missing crate is not.
fn is_transient(error: &reqwest::Error) -> bool {
    match error.status() {
        Some(status) => status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error(),
        None => error.is_connect() || error.is_timeout() || error.is_request() || error.is_body(),
    }
}

/// One line of an index file; only the fields we need.
#[derive(Deserialize)]
struct Release {
    vers: Version,
    yanked: bool,
}

/// Where a crate lives in the index, see
/// <https://doc.rust-lang.org/cargo/reference/registry-index.html#index-files>.
fn index_path(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_paths_follow_the_cargo_layout() {
        assert_eq!(index_path("a"), "1/a");
        assert_eq!(index_path("cc"), "2/cc");
        assert_eq!(index_path("syn"), "3/s/syn");
        assert_eq!(index_path("Serde_JSON"), "se/rd/serde_json");
    }
}
