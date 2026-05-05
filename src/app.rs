use crate::cli::Cli;
use crate::dependency::DependencySection;
use crate::dependency::DependencyUpdate;
use crate::dependency::ManifestDependency;
use crate::dependency::collect_manifest_dependencies;
use crate::error::Result;
use crate::grouped_checklist::ChecklistGroup;
use crate::grouped_checklist::ChecklistItem;
use crate::grouped_checklist::ChecklistSelection;
use crate::grouped_checklist::run_checklist;
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

    let updates = if cli.interactive {
        select_updates_interactively(&plan.updates)?
    } else {
        plan.updates.clone()
    };

    if updates.is_empty() {
        println!("No updates selected.");
        return Ok(());
    }

    manifest.apply(&updates);
    manifest.save()?;
    print_updates(&updates);

    Ok(())
}

fn select_updates_interactively(updates: &[DependencyUpdate]) -> Result<Vec<DependencyUpdate>> {
    let grouped_updates = group_updates_by_section(updates);
    let groups = build_update_checklist_groups(&grouped_updates);
    let selection = run_checklist("Choose dependency upgrades", groups)?;

    Ok(selected_updates_from_selection(
        &grouped_updates,
        &selection,
    ))
}

fn group_updates_by_section(
    updates: &[DependencyUpdate],
) -> Vec<(DependencySection, Vec<DependencyUpdate>)> {
    let mut grouped_updates: Vec<(DependencySection, Vec<DependencyUpdate>)> = Vec::new();

    for update in updates {
        if let Some((_, section_updates)) = grouped_updates
            .iter_mut()
            .find(|(section, _)| *section == update.section)
        {
            section_updates.push(update.clone());
            continue;
        }

        grouped_updates.push((update.section, vec![update.clone()]));
    }

    grouped_updates.sort_by_key(|(section, _)| *section);
    grouped_updates
}

fn build_update_checklist_groups(
    grouped_updates: &[(DependencySection, Vec<DependencyUpdate>)],
) -> Vec<ChecklistGroup> {
    grouped_updates
        .iter()
        .map(|(section, updates)| build_update_checklist_group(*section, updates))
        .collect()
}

fn build_update_checklist_group(
    section: DependencySection,
    updates: &[DependencyUpdate],
) -> ChecklistGroup {
    ChecklistGroup {
        title: section.display_name().to_owned(),
        items: updates.iter().map(build_update_checklist_item).collect(),
    }
}

fn build_update_checklist_item(update: &DependencyUpdate) -> ChecklistItem {
    ChecklistItem {
        label: update.name.clone(),
        current: update.current_requirement.clone(),
        target: update.target_requirement.clone(),
        impact: classify_update_impact(update).to_owned(),
    }
}

fn classify_update_impact(update: &DependencyUpdate) -> &'static str {
    classify_requirement_change(&update.current_requirement, &update.target_requirement)
}

fn classify_requirement_change(current: &str, target: &str) -> &'static str {
    let Some(current) = parse_requirement_version(current) else {
        return "unknown";
    };
    let Some(target) = parse_requirement_version(target) else {
        return "unknown";
    };

    if target.major != current.major {
        "major"
    } else if target.minor != current.minor {
        "minor"
    } else if target.patch != current.patch {
        "patch"
    } else {
        "same"
    }
}

fn parse_requirement_version(requirement: &str) -> Option<semver::Version> {
    let version = requirement
        .trim()
        .trim_start_matches(['^', '~', '='])
        .split([',', ' '])
        .next()?
        .trim();

    let normalized = match version.matches('.').count() {
        0 => format!("{version}.0.0"),
        1 => format!("{version}.0"),
        _ => version.to_string(),
    };

    semver::Version::parse(&normalized).ok()
}

fn selected_updates_from_selection(
    grouped_updates: &[(DependencySection, Vec<DependencyUpdate>)],
    selection: &[ChecklistSelection],
) -> Vec<DependencyUpdate> {
    selection
        .iter()
        .filter_map(|selection| {
            grouped_updates
                .get(selection.group_index)
                .and_then(|(_, updates)| updates.get(selection.item_index))
                .cloned()
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::build_update_checklist_groups;
    use super::group_updates_by_section;
    use super::selected_updates_from_selection;
    use crate::dependency::DependencySection;
    use crate::dependency::DependencyUpdate;
    use crate::grouped_checklist::ChecklistSelection;

    fn update(
        name: &str,
        section: DependencySection,
        current: &str,
        target: &str,
    ) -> DependencyUpdate {
        DependencyUpdate::new(
            name.to_string(),
            section,
            current.to_string(),
            target.to_string(),
        )
    }

    #[test]
    fn groups_updates_by_section_in_stable_order() {
        let updates = vec![
            update("serde", DependencySection::DevDependencies, "1.0", "1.1"),
            update("tokio", DependencySection::Dependencies, "1.0", "1.1"),
            update("cc", DependencySection::BuildDependencies, "1.0", "1.1"),
            update("clap", DependencySection::Dependencies, "4.0", "4.1"),
        ];

        let grouped = group_updates_by_section(&updates);

        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[0].0, DependencySection::Dependencies);
        assert_eq!(
            grouped[0]
                .1
                .iter()
                .map(|it| it.name.as_str())
                .collect::<Vec<_>>(),
            vec!["tokio", "clap"]
        );
        assert_eq!(grouped[1].0, DependencySection::DevDependencies);
        assert_eq!(
            grouped[1]
                .1
                .iter()
                .map(|it| it.name.as_str())
                .collect::<Vec<_>>(),
            vec!["serde"]
        );
        assert_eq!(grouped[2].0, DependencySection::BuildDependencies);
        assert_eq!(
            grouped[2]
                .1
                .iter()
                .map(|it| it.name.as_str())
                .collect::<Vec<_>>(),
            vec!["cc"]
        );
    }

    #[test]
    fn builds_checklist_groups_from_updates() {
        let grouped = vec![
            (
                DependencySection::Dependencies,
                vec![update(
                    "tokio",
                    DependencySection::Dependencies,
                    "1.0",
                    "1.1",
                )],
            ),
            (
                DependencySection::DevDependencies,
                vec![update(
                    "serde",
                    DependencySection::DevDependencies,
                    "1.0",
                    "1.1",
                )],
            ),
        ];

        let groups = build_update_checklist_groups(&grouped);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].title, "dependencies");
        assert_eq!(groups[0].items[0].label, "tokio");
        assert_eq!(groups[0].items[0].current, "1.0");
        assert_eq!(groups[0].items[0].target, "1.1");
        assert_eq!(groups[0].items[0].impact, "minor");
        assert_eq!(groups[1].title, "devDependencies");
        assert_eq!(groups[1].items[0].label, "serde");
    }

    #[test]
    fn maps_selection_indices_back_to_updates() {
        let grouped = vec![
            (
                DependencySection::Dependencies,
                vec![
                    update("tokio", DependencySection::Dependencies, "1.0", "1.1"),
                    update("clap", DependencySection::Dependencies, "4.0", "4.1"),
                ],
            ),
            (
                DependencySection::DevDependencies,
                vec![update(
                    "serde",
                    DependencySection::DevDependencies,
                    "1.0",
                    "1.1",
                )],
            ),
        ];
        let selection = vec![
            ChecklistSelection {
                group_index: 0,
                item_index: 1,
            },
            ChecklistSelection {
                group_index: 1,
                item_index: 0,
            },
        ];

        let updates = selected_updates_from_selection(&grouped, &selection);

        assert_eq!(
            updates
                .iter()
                .map(|it| it.name.as_str())
                .collect::<Vec<_>>(),
            vec!["clap", "serde"]
        );
    }

    #[test]
    fn returns_empty_updates_for_empty_selection() {
        let grouped = vec![(
            DependencySection::Dependencies,
            vec![update(
                "tokio",
                DependencySection::Dependencies,
                "1.0",
                "1.1",
            )],
        )];

        let updates = selected_updates_from_selection(&grouped, &[]);

        assert!(updates.is_empty());
    }
}
