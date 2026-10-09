//! Reading dependencies from `Cargo.toml` and writing requirements back.
//!
//! Edits go through `toml_edit`, so comments, ordering and formatting survive.

use std::{fmt, fs, iter, path::PathBuf};

use anyhow::{Context, Result};
use toml_edit::{DocumentMut, Item, Key, TableLike, Value};

use crate::requirement::Requirement;

pub struct Manifest {
    path: PathBuf,
    document: DocumentMut,
}

impl Manifest {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let document = text
            .parse()
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(Self { path, document })
    }

    pub fn save(&self) -> Result<()> {
        fs::write(&self.path, self.document.to_string())
            .with_context(|| format!("failed to write {}", self.path.display()))
    }

    /// Every upgradable dependency, in manifest order.
    pub fn dependencies(&self) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for section in self.sections() {
            if let Some(table) = self.table(&section) {
                dependencies.extend(
                    table
                        .iter()
                        .filter_map(|(name, item)| Dependency::parse(&section, name, item)),
                );
            }
        }
        dependencies
    }

    /// Rewrites the requirement of a dependency read from this manifest,
    /// keeping the whitespace and comments around it.
    pub fn set_requirement(&mut self, dependency: &Dependency, requirement: &Requirement) {
        let value = self
            .table_mut(&dependency.section)
            .and_then(|table| table.get_mut(&dependency.name))
            .and_then(version_mut)
            .expect("dependency comes from this manifest");
        let decor = value.decor().clone();
        *value = requirement.to_string().into();
        *value.decor_mut() = decor;
    }

    /// All sections that may declare dependencies, whether present or not.
    fn sections(&self) -> Vec<Section> {
        let targets = self
            .document
            .get("target")
            .and_then(Item::as_table_like)
            .into_iter()
            .flat_map(|targets| targets.iter().map(|(target, _)| Some(target.to_owned())));

        iter::once(None)
            .chain(targets)
            .flat_map(|target| {
                Kind::ALL.map(|kind| Section {
                    kind,
                    target: target.clone(),
                })
            })
            .collect()
    }

    fn table(&self, section: &Section) -> Option<&dyn TableLike> {
        let parent = match &section.target {
            Some(target) => self.document.get("target")?.get(target)?,
            None => self.document.as_item(),
        };
        parent.get(section.kind.key())?.as_table_like()
    }

    fn table_mut(&mut self, section: &Section) -> Option<&mut dyn TableLike> {
        let parent = match &section.target {
            Some(target) => self.document.get_mut("target")?.get_mut(target)?,
            None => self.document.as_item_mut(),
        };
        parent.get_mut(section.kind.key())?.as_table_like_mut()
    }
}

/// A crates.io dependency as declared in one section of the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub section: Section,
    /// The key in the manifest.
    pub name: String,
    /// The crate on crates.io, which differs from `name` when renamed with `package`.
    pub package: String,
    pub requirement: Requirement,
}

impl Dependency {
    /// Keys that point a dependency somewhere other than crates.io.
    const ELSEWHERE: [&str; 4] = ["workspace", "path", "git", "registry"];

    fn parse(section: &Section, name: &str, item: &Item) -> Option<Self> {
        let (version, package) = match item.as_table_like() {
            Some(table) if Self::ELSEWHERE.iter().any(|key| table.contains_key(key)) => {
                return None;
            }
            Some(table) => (
                table.get("version")?.as_str()?,
                table.get("package").and_then(Item::as_str),
            ),
            None => (item.as_str()?, None),
        };
        Some(Self {
            section: section.clone(),
            name: name.to_owned(),
            package: package.unwrap_or(name).to_owned(),
            requirement: Requirement::parse(version)?,
        })
    }
}

/// The `version` value of a dependency, whether written as `"1.0"` or `{ version = "1.0" }`.
fn version_mut(item: &mut Item) -> Option<&mut Value> {
    if item.is_table_like() {
        item.as_table_like_mut()?.get_mut("version")?.as_value_mut()
    } else {
        item.as_value_mut()
    }
}

/// A dependency table, such as `[dev-dependencies]` or `[target.'cfg(unix)'.dependencies]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub kind: Kind,
    /// The platform of a `[target.<platform>]` table.
    pub target: Option<String>,
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.target {
            Some(target) => {
                let target = Key::new(target.as_str());
                write!(f, "target.{}.{}", target.display_repr(), self.kind.key())
            }
            None => f.write_str(self.kind.key()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Normal,
    Development,
    Build,
}

impl Kind {
    const ALL: [Self; 3] = [Self::Normal, Self::Development, Self::Build];

    fn key(self) -> &'static str {
        match self {
            Self::Normal => "dependencies",
            Self::Development => "dev-dependencies",
            Self::Build => "build-dependencies",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
[package]
name = "demo"

[dependencies]
serde = "1.0.200" # keep me
tokio = { version = "~1.44", features = ["rt"] }
json = { package = "serde_json", version = "1" }
local = { path = "../local", version = "0.1" }
shared = { workspace = true }
anything = "*"

[dependencies.clap]
version = "4.5.0"
features = ["derive"]

[dev-dependencies]
insta = "1.40"

[target.'cfg(unix)'.dependencies]
libc = "0.2.150"
"#;

    fn manifest() -> Manifest {
        Manifest {
            path: PathBuf::from("Cargo.toml"),
            document: MANIFEST.parse().unwrap(),
        }
    }

    #[test]
    fn collects_upgradable_dependencies_from_every_section() {
        let found: Vec<_> = manifest()
            .dependencies()
            .iter()
            .map(|d| format!("{} {}={} {}", d.section, d.name, d.package, d.requirement))
            .collect();
        assert_eq!(
            found,
            [
                "dependencies serde=serde 1.0.200",
                "dependencies tokio=tokio ~1.44",
                "dependencies json=serde_json 1",
                "dependencies clap=clap 4.5.0",
                "dev-dependencies insta=insta 1.40",
                r#"target."cfg(unix)".dependencies libc=libc 0.2.150"#,
            ]
        );
    }

    #[test]
    fn rewrites_only_the_requirement() {
        let mut manifest = manifest();
        for dependency in manifest.dependencies() {
            let requirement = Requirement::parse(match dependency.name.as_str() {
                "serde" => "1.0.229",
                "tokio" => "~1.52",
                "json" => "2",
                "clap" => "4.6.7",
                "insta" => "1.44",
                _ => "0.2.180",
            })
            .unwrap();
            manifest.set_requirement(&dependency, &requirement);
        }

        let expected = MANIFEST
            .replace(r#""1.0.200""#, r#""1.0.229""#)
            .replace(r#""~1.44""#, r#""~1.52""#)
            .replace(r#"version = "1" }"#, r#"version = "2" }"#)
            .replace(r#""4.5.0""#, r#""4.6.7""#)
            .replace(r#""1.40""#, r#""1.44""#)
            .replace(r#""0.2.150""#, r#""0.2.180""#);
        assert_eq!(manifest.document.to_string(), expected);
    }
}
