use crate::dependency::DependencyCandidate;
use futures::StreamExt;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct RegistryLookup {
    pub(crate) candidate: DependencyCandidate,
    pub(crate) latest_versions: std::result::Result<Vec<String>, String>,
}

pub(crate) async fn fetch_latest_versions(
    client: Arc<crates_io_api::AsyncClient>,
    dependencies: Vec<DependencyCandidate>,
) -> Vec<RegistryLookup> {
    let mut tasks = futures::stream::FuturesUnordered::new();

    for dependency in dependencies {
        let client = client.clone();
        tasks.push(async move {
            let crate_name = dependency.crate_name.clone();
            let latest_versions = match client.get_crate(&crate_name).await {
                Ok(crate_info) => Ok(crate_info
                    .versions
                    .into_iter()
                    .map(|version| version.num)
                    .collect()),
                Err(err) => Err(err.to_string()),
            };

            RegistryLookup {
                candidate: dependency,
                latest_versions,
            }
        });
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.next().await {
        results.push(result);
    }
    results
}
