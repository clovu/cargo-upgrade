mod cli;
mod manifest;
mod registry;
mod requirement;
mod upgrade;

use anyhow::{Result, bail};
use clap::Parser;

use crate::{
    cli::Cli,
    manifest::{Dependency, Manifest},
    registry::CratesIo,
    upgrade::Upgrade,
};

#[tokio::main]
async fn main() -> Result<()> {
    let Cli::Upgrade(args) = Cli::parse();

    let mut manifest = Manifest::load(&args.manifest_path)?;
    let dependencies = select(manifest.dependencies(), &args.crates)?;

    let releases = CratesIo::new()
        .releases(dependencies.iter().map(|d| d.package.as_str()))
        .await;
    for error in releases.values().filter_map(|r| r.as_ref().err()) {
        eprintln!("warning: {error:#}");
    }

    let upgrades = upgrade::plan(&dependencies, &releases, args.policy());
    if upgrades.is_empty() {
        println!("All dependencies are up to date.");
        return Ok(());
    }
    print(&upgrades);

    let path = args.manifest_path.display();
    if args.dry_run {
        println!("\nDry run: {path} was not modified.");
    } else {
        for upgrade in &upgrades {
            manifest.set_requirement(upgrade.dependency, &upgrade.to);
        }
        manifest.save()?;
        println!("\nUpdated {path}.");
    }
    Ok(())
}

/// Keeps the dependencies named on the command line, or all of them when none are.
fn select(mut dependencies: Vec<Dependency>, names: &[String]) -> Result<Vec<Dependency>> {
    if let Some(missing) = names
        .iter()
        .find(|n| !dependencies.iter().any(|d| d.name == **n))
    {
        bail!("no upgradable dependency named `{missing}`");
    }
    if !names.is_empty() {
        dependencies.retain(|d| names.contains(&d.name));
    }
    Ok(dependencies)
}

/// Prints the upgrades grouped by section, in manifest order.
fn print(upgrades: &[Upgrade]) {
    let width = |f: fn(&Upgrade) -> usize| upgrades.iter().map(f).max().unwrap_or(0);
    let name_width = width(|u| u.dependency.name.len());
    let from_width = width(|u| u.dependency.requirement.to_string().len());

    let mut section = None;
    for upgrade in upgrades {
        let dependency = upgrade.dependency;
        if section != Some(&dependency.section) {
            if section.is_some() {
                println!();
            }
            println!("{}", dependency.section);
            section = Some(&dependency.section);
        }
        let from = dependency.requirement.to_string();
        let note = if upgrade.is_breaking() {
            "  (breaking)"
        } else {
            ""
        };
        println!(
            "  {:name_width$}  {from:>from_width$}  →  {}{note}",
            dependency.name, upgrade.to
        );
    }
}
