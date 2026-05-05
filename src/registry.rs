use crate::dependency::ManifestDependency;
use futures::StreamExt;
use semver::Version;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct VersionResolution {
    pub(crate) dependency: ManifestDependency,
    pub(crate) releases: std::result::Result<Vec<Version>, String>,
}

pub(crate) trait ReleaseFetchProgress {
    fn dependency_finished(&self, dependency: &ManifestDependency);
}

impl ReleaseFetchProgress for () {
    fn dependency_finished(&self, _dependency: &ManifestDependency) {}
}

pub(crate) async fn fetch_available_releases<P>(
    client: Arc<crates_io_api::AsyncClient>,
    dependencies: Vec<ManifestDependency>,
    progress: &P,
) -> Vec<VersionResolution>
where
    P: ReleaseFetchProgress + ?Sized,
{
    let mut tasks = futures::stream::FuturesUnordered::new();

    for dependency in dependencies {
        let client = client.clone();

        tasks.push(async move {
            let releases = match client.get_crate(&dependency.name).await {
                Ok(crate_info) => Ok(crate_info
                    .versions
                    .into_iter()
                    .filter_map(|version| Version::parse(&version.num).ok())
                    .collect()),
                Err(error) => Err(error.to_string()),
            };

            VersionResolution {
                dependency,
                releases,
            }
        });
    }

    let mut resolutions = Vec::new();
    while let Some(resolution) = tasks.next().await {
        progress.dependency_finished(&resolution.dependency);
        resolutions.push(resolution);
    }
    resolutions
}
