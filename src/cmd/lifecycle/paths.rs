use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::load::{find_clause_toml, find_rfc_toml, split_clause_id};
use std::path::PathBuf;

fn legacy_rfc_json_path(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let path = config.rfc_source_path(rfc_id, "json");
    path.exists().then_some(path)
}

fn legacy_clause_json_path(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let (rfc_id, clause_name) = split_clause_id(clause_id)?;
    let path = config.clause_source_path(rfc_id, clause_name, "json");
    path.exists().then_some(path)
}

pub(super) fn require_rfc_toml_path(config: &Config, rfc_id: &str) -> DiagnosticResult<PathBuf> {
    require_toml_path(
        || find_rfc_toml(config, rfc_id),
        || legacy_rfc_json_path(config, rfc_id).is_some(),
        || {
            Diagnostic::new(
                DiagnosticCode::E0505MigrationRequired,
                format!(
                    "Legacy JSON RFC exists for {rfc_id}; run `govctl migrate` before RFC lifecycle commands."
                ),
                rfc_id,
            )
        },
        || {
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {rfc_id}"),
                rfc_id,
            )
        },
    )
}

pub(super) fn require_replacement_rfc_toml_path(
    config: &Config,
    rfc_id: &str,
) -> DiagnosticResult<PathBuf> {
    require_replacement_path(
        || require_rfc_toml_path(config, rfc_id),
        DiagnosticCode::E0102RfcNotFound,
        || {
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("Replacement RFC not found: {rfc_id}"),
                rfc_id,
            )
        },
    )
}

pub(super) fn require_clause_toml_path(
    config: &Config,
    clause_id: &str,
) -> DiagnosticResult<PathBuf> {
    require_toml_path(
        || find_clause_toml(config, clause_id),
        || legacy_clause_json_path(config, clause_id).is_some(),
        || {
            Diagnostic::new(
                DiagnosticCode::E0505MigrationRequired,
                format!(
                    "Legacy JSON clause exists for {clause_id}; run `govctl migrate` before clause lifecycle commands."
                ),
                clause_id,
            )
        },
        || {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {clause_id}"),
                clause_id,
            )
        },
    )
}

pub(super) fn require_replacement_clause_toml_path(
    config: &Config,
    clause_id: &str,
) -> DiagnosticResult<PathBuf> {
    require_replacement_path(
        || require_clause_toml_path(config, clause_id),
        DiagnosticCode::E0202ClauseNotFound,
        || {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Replacement clause not found: {clause_id}"),
                clause_id,
            )
        },
    )
}

fn require_replacement_path<F, NotFound>(
    lookup: F,
    not_found_code: DiagnosticCode,
    not_found: NotFound,
) -> DiagnosticResult<PathBuf>
where
    F: FnOnce() -> DiagnosticResult<PathBuf>,
    NotFound: FnOnce() -> Diagnostic,
{
    lookup().map_err(|err| {
        if err.code == not_found_code {
            not_found()
        } else {
            err
        }
    })
}

fn require_toml_path<Find, HasLegacy, MigrationRequired, NotFound>(
    find: Find,
    has_legacy: HasLegacy,
    migration_required: MigrationRequired,
    not_found: NotFound,
) -> DiagnosticResult<PathBuf>
where
    Find: FnOnce() -> Option<PathBuf>,
    HasLegacy: FnOnce() -> bool,
    MigrationRequired: FnOnce() -> Diagnostic,
    NotFound: FnOnce() -> Diagnostic,
{
    if let Some(path) = find() {
        return Ok(path);
    }
    if has_legacy() {
        return Err(migration_required());
    }
    Err(not_found())
}
