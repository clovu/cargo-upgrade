#[derive(Clone, Copy, Debug)]
pub(crate) enum CargoTable {
    Dependencies,
    DevDependencies,
    BuildDependencies,
}

impl CargoTable {
    pub(crate) fn toml_key(self) -> &'static str {
        match self {
            CargoTable::Dependencies => "dependencies",
            CargoTable::DevDependencies => "dev-dependencies",
            CargoTable::BuildDependencies => "build-dependencies",
        }
    }
}

#[derive(Debug)]
pub(crate) struct DependencyCandidate {
    pub(crate) crate_name: String,
    pub(crate) current_requirement: String,
    pub(crate) table: CargoTable,
}

fn dependency_requirement(dep: &cargo_toml::Dependency) -> Option<String> {
    match dep {
        cargo_toml::Dependency::Simple(version) => Some(version.clone()),
        cargo_toml::Dependency::Inherited(_) => None,
        cargo_toml::Dependency::Detailed(detail) => detail.version.clone(),
    }
}

fn collect_from_table(
    cargo_toml: &cargo_toml::Manifest,
    table: CargoTable,
) -> Vec<DependencyCandidate> {
    let dependencies = match table {
        CargoTable::Dependencies => &cargo_toml.dependencies,
        CargoTable::DevDependencies => &cargo_toml.dev_dependencies,
        CargoTable::BuildDependencies => &cargo_toml.build_dependencies,
    };

    dependencies
        .iter()
        .filter_map(|(crate_name, dependency)| {
            let current_requirement = dependency_requirement(dependency)?;
            Some(DependencyCandidate {
                crate_name: crate_name.to_string(),
                current_requirement,
                table,
            })
        })
        .collect()
}

pub(crate) fn collect_upgradable_dependencies(
    cargo_toml: &cargo_toml::Manifest,
) -> Vec<DependencyCandidate> {
    let mut result = Vec::new();
    result.extend(collect_from_table(cargo_toml, CargoTable::Dependencies));
    result.extend(collect_from_table(cargo_toml, CargoTable::DevDependencies));
    result.extend(collect_from_table(
        cargo_toml,
        CargoTable::BuildDependencies,
    ));
    result
}
