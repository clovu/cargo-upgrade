use crate::dependency::CargoTable;
use crate::error::Result;
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

#[derive(Debug)]
pub(crate) struct UpgradeDecision {
    pub(crate) table: CargoTable,
    pub(crate) crate_name: String,
    pub(crate) next_requirement: String,
}

impl UpgradeDecision {
    pub(crate) fn new(table: CargoTable, crate_name: String, next_requirement: String) -> Self {
        Self {
            table,
            crate_name,
            next_requirement,
        }
    }
}

pub(crate) fn apply_updates(doc: &mut DocumentMut, updates: &[UpgradeDecision]) {
    for update in updates {
        doc[update.table.toml_key()][&update.crate_name] =
            toml_edit::value(update.next_requirement.clone());
    }
}

pub(crate) fn write_manifest_once(path: &Path, doc: &DocumentMut) -> Result<()> {
    fs::write(path, doc.to_string())?;
    Ok(())
}
