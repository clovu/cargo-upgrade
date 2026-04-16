use crate::error::Result;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use toml_edit::DocumentMut;

const CARGO_TOML_PATH: &str = "Cargo.toml";

pub(crate) fn resolve_manifest_path() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    let manifest_path = current_dir.join(CARGO_TOML_PATH);

    let metadata = fs::metadata(&manifest_path)?;
    if !metadata.is_file() {
        return Err(format!("{CARGO_TOML_PATH} not found in the current directory").into());
    }

    Ok(manifest_path)
}

pub(crate) fn load_manifest(path: &Path) -> Result<(DocumentMut, cargo_toml::Manifest)> {
    let cargo_toml_content = fs::read_to_string(path)?;
    let doc = cargo_toml_content.parse::<DocumentMut>()?;
    let cargo_toml = cargo_toml::Manifest::from_path(path)?;
    Ok((doc, cargo_toml))
}
