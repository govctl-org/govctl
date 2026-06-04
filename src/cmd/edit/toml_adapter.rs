use super::adapter::{TomlAdapter, display_scope_for_dir};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{AdrEntry, GuardEntry, WorkItemEntry};
use crate::parse::{load_work_items, write_adr, write_guard, write_work_item};
use crate::write::WriteOp;
use std::path::PathBuf;

fn load_toml_entry<T, Load, LoadError, Matches>(
    config: &Config,
    id: &str,
    scope_dir: PathBuf,
    load: Load,
    matches: Matches,
    missing_code: DiagnosticCode,
    missing_kind: &str,
) -> Result<T, Diagnostic>
where
    Load: FnOnce(&Config) -> Result<Vec<T>, LoadError>,
    LoadError: Into<Diagnostic>,
    Matches: Fn(&T, &str) -> bool,
{
    let scope = display_scope_for_dir(config, scope_dir);
    load(config)
        .map_err(Into::into)?
        .into_iter()
        .find(|entry| matches(entry, id))
        .ok_or_else(|| {
            Diagnostic::new(
                missing_code,
                format!("{missing_kind} not found: {id}"),
                &scope,
            )
        })
}

/// ADR TOML adapter.
pub struct AdrTomlAdapter;

impl TomlAdapter for AdrTomlAdapter {
    type Entry = AdrEntry;

    fn load(config: &Config, id: &str) -> DiagnosticResult<Self::Entry> {
        crate::artifact_catalog::load_adr_by_id(config, id)
    }

    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> DiagnosticResult<()> {
        write_adr(
            &entry.path,
            &entry.spec,
            op,
            Some(&config.display_path(&entry.path)),
        )
    }
}

/// Work item TOML adapter.
pub struct WorkTomlAdapter;

impl TomlAdapter for WorkTomlAdapter {
    type Entry = WorkItemEntry;

    fn load(config: &Config, id: &str) -> DiagnosticResult<Self::Entry> {
        if id.starts_with("WI-") {
            match crate::artifact_catalog::load_work_item_by_id(config, id) {
                Ok(entry) => return Ok(entry),
                Err(err) if err.code == DiagnosticCode::E0402WorkNotFound => {}
                Err(err) => return Err(err),
            }
        }

        load_toml_entry(
            config,
            id,
            config.work_dir(),
            load_work_items,
            |entry, id| entry.spec.govctl.id == id || entry.path.to_string_lossy().contains(id),
            DiagnosticCode::E0402WorkNotFound,
            "Work item",
        )
    }

    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> DiagnosticResult<()> {
        write_work_item(
            &entry.path,
            &entry.spec,
            op,
            Some(&config.display_path(&entry.path)),
        )
    }
}

/// Guard TOML adapter.
pub struct GuardTomlAdapter;

impl TomlAdapter for GuardTomlAdapter {
    type Entry = GuardEntry;

    fn load(config: &Config, id: &str) -> DiagnosticResult<Self::Entry> {
        crate::artifact_catalog::load_guard_by_id(config, id)
    }

    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> DiagnosticResult<()> {
        write_guard(
            &entry.path,
            &entry.spec,
            op,
            Some(&config.display_path(&entry.path)),
        )
    }
}
