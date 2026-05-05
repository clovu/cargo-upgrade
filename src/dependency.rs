#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum DependencySection {
    Dependencies,
    DevDependencies,
    BuildDependencies,
}

impl DependencySection {
    pub(crate) fn toml_key(self) -> &'static str {
        match self {
            Self::Dependencies => "dependencies",
            Self::DevDependencies => "dev-dependencies",
            Self::BuildDependencies => "build-dependencies",
        }
    }

    pub(crate) fn display_name(self) -> &'static str {
        match self {
            Self::Dependencies => "dependencies",
            Self::DevDependencies => "devDependencies",
            Self::BuildDependencies => "buildDependencies",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ManifestDependency {
    pub(crate) name: String,
    pub(crate) requirement: String,
    pub(crate) section: DependencySection,
}

#[derive(Clone, Debug)]
pub(crate) struct DependencyUpdate {
    pub(crate) name: String,
    pub(crate) section: DependencySection,
    pub(crate) current_requirement: String,
    pub(crate) target_requirement: String,
}

impl DependencyUpdate {
    pub(crate) fn new(
        name: String,
        section: DependencySection,
        current_requirement: String,
        target_requirement: String,
    ) -> Self {
        Self {
            name,
            section,
            current_requirement,
            target_requirement,
        }
    }
}

fn requirement_of(dependency: &cargo_toml::Dependency) -> Option<String> {
    match dependency {
        cargo_toml::Dependency::Simple(version) => Some(version.clone()),
        cargo_toml::Dependency::Inherited(_) => None,
        cargo_toml::Dependency::Detailed(detail) => detail.version.clone(),
    }
}

fn collect_from_section(
    manifest: &cargo_toml::Manifest,
    section: DependencySection,
) -> Vec<ManifestDependency> {
    let dependencies = match section {
        DependencySection::Dependencies => &manifest.dependencies,
        DependencySection::DevDependencies => &manifest.dev_dependencies,
        DependencySection::BuildDependencies => &manifest.build_dependencies,
    };

    dependencies
        .iter()
        .filter_map(|(name, dependency)| {
            let requirement = requirement_of(dependency)?;
            Some(ManifestDependency {
                name: name.to_string(),
                requirement,
                section,
            })
        })
        .collect()
}

pub(crate) fn collect_manifest_dependencies(
    manifest: &cargo_toml::Manifest,
) -> Vec<ManifestDependency> {
    let mut dependencies = Vec::new();
    dependencies.extend(collect_from_section(
        manifest,
        DependencySection::Dependencies,
    ));
    dependencies.extend(collect_from_section(
        manifest,
        DependencySection::DevDependencies,
    ));
    dependencies.extend(collect_from_section(
        manifest,
        DependencySection::BuildDependencies,
    ));
    dependencies
}
