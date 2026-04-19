use crate::cli::Cli;
use crate::dependency::DependencyUpdate;
use crate::dependency::ManifestDependency;
use crate::dependency::collect_manifest_dependencies;
use crate::error::Result;
use crate::manifest::LoadedManifest;
use crate::registry::fetch_available_releases;
use crate::versioning::choose_target_release;
use crate::versioning::rewrite_requirement;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

fn filter_dependencies(
    mut dependencies: Vec<ManifestDependency>,
    crates: &[String],
) -> Vec<ManifestDependency> {
    if crates.is_empty() {
        dependencies
    } else {
        let crates: std::collections::HashSet<_> = crates.iter().map(|it| it.trim()).collect();

        dependencies.retain(|it| crates.contains(it.name.trim()));

        dependencies
    }
}

fn collect_filtered_dependencies(
    manifest: &cargo_toml::Manifest,
    crates: &[String],
) -> Vec<ManifestDependency> {
    filter_dependencies(collect_manifest_dependencies(manifest), crates)
}

pub(crate) async fn run(cli: Cli) -> ExitCode {
    match try_run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn try_run(cli: Cli) -> Result<()> {
    println!("input options: {cli:#?}");
    ensure_supported_options(&cli)?;

    let mut manifest = LoadedManifest::load()?;
    let dependencies = collect_filtered_dependencies(manifest.manifest(), &cli.crates);
    let releases = fetch_available_releases(Arc::new(new_registry_client()?), dependencies).await;
    let plan = build_upgrade_plan(releases, cli.latest)?;

    print_failures(&plan.failures);

    if plan.updates.is_empty() {
        println!("No updates.");
        return Ok(());
    }

    if cli.dry_run {
        print_updates(&plan.updates);
        println!();
        println!("Dry run. Cargo.toml was not modified.");
        return Ok(());
    }

    manifest.apply(&plan.updates);
    manifest.save()?;
    print_updates(&plan.updates);

    Ok(())
}

struct UpgradePlan {
    updates: Vec<DependencyUpdate>,
    failures: Vec<String>,
}

fn build_upgrade_plan(
    resolutions: Vec<crate::registry::VersionResolution>,
    use_latest: bool,
) -> Result<UpgradePlan> {
    let mut updates = Vec::new();
    let mut failures = Vec::new();

    for resolution in resolutions {
        let dependency = resolution.dependency;

        match resolution.releases {
            Ok(releases) => {
                if let Some(release) =
                    choose_target_release(&dependency.requirement, releases, use_latest)?
                {
                    let target_requirement = rewrite_requirement(&dependency.requirement, &release);
                    updates.push(DependencyUpdate::new(
                        dependency.name,
                        dependency.section,
                        dependency.requirement,
                        target_requirement,
                    ));
                }
            }
            Err(error) => {
                failures.push(format!(
                    "failed to fetch releases for {}: {}",
                    dependency.name, error
                ));
            }
        }
    }

    Ok(UpgradePlan { updates, failures })
}

fn ensure_supported_options(cli: &Cli) -> Result<()> {
    let mut unsupported = Vec::new();

    if cli.recursive {
        unsupported.push("--recursive");
    }
    if cli.global {
        unsupported.push("--global");
    }
    if cli.workspace {
        unsupported.push("--workspace");
    }
    if cli.prod {
        unsupported.push("--prod");
    }
    if cli.dev {
        unsupported.push("--dev");
    }
    if cli.no_optional {
        unsupported.push("--no-optional");
    }
    if cli.interactive {
        unsupported.push("--interactive");
    }
    if !cli.filter.is_empty() {
        unsupported.push("--filter");
    }
    if cli.depth.is_some() {
        unsupported.push("--depth");
    }

    if unsupported.is_empty() {
        return Ok(());
    }

    Err(format!("unsupported options: {}", unsupported.join(", ")).into())
}

fn print_updates(updates: &[DependencyUpdate]) {
    for update in updates {
        println!(
            "{} {} -> {}",
            update.name, update.current_requirement, update.target_requirement
        );
    }
}

fn print_failures(failures: &[String]) {
    for failure in failures {
        eprintln!("{failure}");
    }
}

fn new_registry_client() -> Result<crates_io_api::AsyncClient> {
    Ok(crates_io_api::AsyncClient::new(
        &format!(
            "{}/{} (https://github.com/clovu/cargo-upgrade; {})",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            "hi@clovu.me"
        ),
        Duration::from_millis(100),
    )?)
}
