use crate::dependency::DependencyUpdate;
use crate::error::Result;
use std::fs;
use std::path::PathBuf;
use toml_edit::DocumentMut;

const MANIFEST_NAME: &str = "Cargo.toml";

pub(crate) struct LoadedManifest {
    path: PathBuf,
    document: DocumentMut,
    manifest: cargo_toml::Manifest,
}

impl LoadedManifest {
    pub(crate) fn load() -> Result<Self> {
        let path = resolve_manifest_path()?;
        let content = fs::read_to_string(&path)?;

        Ok(Self {
            manifest: cargo_toml::Manifest::from_path(&path)?,
            document: content.parse::<DocumentMut>()?,
            path,
        })
    }

    pub(crate) fn manifest(&self) -> &cargo_toml::Manifest {
        &self.manifest
    }

    pub(crate) fn apply(&mut self, updates: &[DependencyUpdate]) {
        for update in updates {
            let dependency = &mut self.document[update.section.toml_key()][&update.name];

            if let Some(inline_table) = dependency.as_inline_table_mut() {
                inline_table.insert(
                    "version",
                    toml_edit::Value::from(update.target_requirement.clone()),
                );
                continue;
            }

            if let Some(table) = dependency.as_table_like_mut() {
                table.insert(
                    "version",
                    toml_edit::value(update.target_requirement.clone()),
                );
                continue;
            }

            *dependency = toml_edit::value(update.target_requirement.clone());
        }
    }

    pub(crate) fn save(&self) -> Result<()> {
        fs::write(&self.path, self.document.to_string())?;
        Ok(())
    }
}

fn resolve_manifest_path() -> Result<PathBuf> {
    let path = std::env::current_dir()?.join(MANIFEST_NAME);
    let metadata = fs::metadata(&path)?;

    if !metadata.is_file() {
        return Err(format!("no {MANIFEST_NAME} found in the current directory").into());
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::LoadedManifest;
    use crate::dependency::DependencySection;
    use crate::dependency::DependencyUpdate;
    use toml_edit::DocumentMut;

    #[test]
    fn preserves_inline_table_fields_when_applying_updates() {
        let path = std::env::temp_dir().join(format!(
            "cargo-upgrade-test-{}-Cargo.toml",
            std::process::id()
        ));

        std::fs::write(
            &path,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\ntokio = { version = \"1.0\", features = [\"macros\"] }\n",
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let mut manifest = LoadedManifest {
            manifest: cargo_toml::Manifest::from_path(&path).unwrap(),
            document: content.parse::<DocumentMut>().unwrap(),
            path: path.clone(),
        };

        manifest.apply(&[DependencyUpdate::new(
            "tokio".into(),
            DependencySection::Dependencies,
            "1.0".into(),
            "1.44.2".into(),
        )]);

        let dependency = &manifest.document["dependencies"]["tokio"];
        assert_eq!(dependency["version"].as_str(), Some("1.44.2"));
        assert!(dependency.to_string().contains("features"));

        let _ = std::fs::remove_file(path);
    }
}
