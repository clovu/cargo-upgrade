mod dependency;
mod error;
mod manifest;
mod registry;
mod update;
mod versioning;

use std::sync::Arc;
use std::time::Duration;

use error::Result;
use manifest::{load_manifest, resolve_manifest_path};
use registry::fetch_latest_versions;
use update::{UpgradeDecision, apply_updates, write_manifest_once};
use versioning::select_target_version;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    run().await
}

async fn run() -> Result<()> {
    let user_agent = build_user_agent();
    let manifest_path = resolve_manifest_path()?;

    let (mut doc, cargo_toml) = load_manifest(&manifest_path)?;
    let dependencies = dependency::collect_upgradable_dependencies(&cargo_toml);

    let client = crates_io_api::AsyncClient::new(&user_agent, Duration::from_millis(100))?;
    let lookup_results = fetch_latest_versions(Arc::new(client), dependencies).await;

    let mut updates: Vec<UpgradeDecision> = Vec::new();
    let mut errors = Vec::new();

    for lookup in lookup_results {
        let crate_name = lookup.candidate.crate_name;
        let current_requirement = lookup.candidate.current_requirement;
        let table = lookup.candidate.table;

        match lookup.latest_versions {
            Ok(versions) => {
                if let Some(next_requirement) =
                    select_target_version(&current_requirement, versions)?
                {
                    updates.push(UpgradeDecision::new(table, crate_name, next_requirement));
                }
            }
            Err(err) => {
                errors.push(format!("Failed to fetch crate '{crate_name}': {err}"));
            }
        }
    }

    for err in errors {
        eprintln!("{err}");
    }

    if !updates.is_empty() {
        apply_updates(&mut doc, &updates);
        write_manifest_once(&manifest_path, &doc)?;
    }

    Ok(())
}

fn build_user_agent() -> String {
    format!(
        "{}/{} (https://github.com/clovu/cargo-upgrade; {})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        "hi@clovu.me"
    )
}
