//! Deciding which dependencies to upgrade, and to what.

use crate::{
    manifest::Dependency,
    registry::Releases,
    requirement::{Policy, Requirement},
};

pub struct Upgrade<'a> {
    pub dependency: &'a Dependency,
    pub to: Requirement,
}

impl Upgrade<'_> {
    pub fn is_breaking(&self) -> bool {
        self.dependency.requirement.is_breaking(&self.to)
    }
}

/// Upgrades for every dependency whose releases are known and allow one.
pub fn plan<'a>(
    dependencies: &'a [Dependency],
    releases: &Releases,
    policy: Policy,
) -> Vec<Upgrade<'a>> {
    dependencies
        .iter()
        .filter_map(|dependency| {
            let versions = releases.get(dependency.package.as_str())?.as_ref().ok()?;
            let to = dependency.requirement.upgrade(versions, policy)?;
            Some(Upgrade { dependency, to })
        })
        .collect()
}
