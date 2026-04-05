mod error;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use error::Result;
use futures::StreamExt;
use toml_edit::DocumentMut;

#[tokio::main]
async fn main() -> Result<()> {
    // load env profile
    dotenvy::dotenv()?;

    let user_agent = generate_user_agent();

    discover_cargo_configuration()?;

    let cargo_toml_path = discover_cargo_configuration()?;
    let cargo_toml_content = fs::read_to_string(&cargo_toml_path)?;

    let mut doc = cargo_toml_content.parse::<DocumentMut>()?;

    let cargo_toml = cargo_toml::Manifest::from_path(&cargo_toml_path)?;
    let dependencies = cargo_toml.dependencies;

    let client = crates_io_api::AsyncClient::new(&user_agent, Duration::from_millis(100))?;
    let client_arc = Arc::new(client);

    let mut tasks = futures::stream::FuturesUnordered::new();

    for (dep, deps) in dependencies {
        let version = match deps {
            cargo_toml::Dependency::Simple(version) => Some(version),
            // Inherited dependencies are those that are inherited from a workspace or a parent manifest.
            cargo_toml::Dependency::Inherited(_) => continue,
            cargo_toml::Dependency::Detailed(dependency_detail) => dependency_detail.version,
        };

        let version = match version {
            Some(version) => version,
            None => continue,
        };

        let client = client_arc.clone();
        tasks.push(async move {
            let latest_crate = client.get_crate(&dep).await;
            (latest_crate, dep, version)
        });
    }

    while let Some((Ok(crate_info), crate_name, current_version)) = tasks.next().await {
        let mut versions: Vec<_> = crate_info.versions.iter().collect();

        versions.sort_by(|a, b| {
            let version_a = semver::Version::parse(&a.num).ok();
            let version_b = semver::Version::parse(&b.num).ok();
            version_a.cmp(&version_b)
        });

        let current_version_req = current_version.parse::<semver::VersionReq>()?;

        let Some(matching_version) = versions.into_iter().rfind(|version| {
            let Ok(version) = version.num.parse::<semver::Version>() else {
                return false;
            };
            let Ok(version_req) = version.to_string().parse::<semver::VersionReq>() else {
                return false;
            };
            if current_version_req.eq(&version_req) {
                return false;
            }
            current_version_req.matches(&version)
        }) else {
            continue;
        };

        let mut final_version = matching_version.num.to_string();

        if current_version.starts_with("~") {
            final_version = format!("~{}", matching_version.num);
        } else if !current_version_req.comparators.is_empty() || current_version.trim().eq("*") {
            final_version = format!("^{}", matching_version.num);
        }

        println!("{:#?}", matching_version);

        doc["dependencies"][&crate_name] = toml_edit::value(final_version);
        fs::write(&cargo_toml_path, doc.to_string())?;
    }

    Ok(())
}

fn generate_user_agent() -> String {
    format!(
        "{}/{} (https://github.com/clovu/cargo-upgrade; {})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        "hi@clovu.me"
    )
}

const CARGO_TOML_PATH: &str = "Cargo-test.toml";

fn discover_cargo_configuration() -> Result<PathBuf> {
    // Implementation for discovering cargo configuration
    let current_dir = std::env::current_dir()?;
    let cargo_toml_path = current_dir.join(CARGO_TOML_PATH);

    let metadata = fs::metadata(&cargo_toml_path)?;
    if !metadata.is_file() {
        return Err("Cargo.toml not found in the current directory".into());
    }
    Ok(cargo_toml_path)
}
