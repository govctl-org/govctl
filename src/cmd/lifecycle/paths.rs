use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::load::{find_clause_toml, find_rfc_toml, split_clause_id};
use std::path::PathBuf;

fn legacy_rfc_json_path(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let path = config.rfc_dir().join(rfc_id).join("rfc.json");
    path.exists().then_some(path)
}

fn legacy_clause_json_path(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let (rfc_id, clause_name) = split_clause_id(clause_id)?;
    let path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.json"));
    path.exists().then_some(path)
}

pub(super) fn require_rfc_toml_path(config: &Config, rfc_id: &str) -> DiagnosticResult<PathBuf> {
    if let Some(path) = find_rfc_toml(config, rfc_id) {
        return Ok(path);
    }
    if legacy_rfc_json_path(config, rfc_id).is_some() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0505MigrationRequired,
            format!(
                "Legacy JSON RFC exists for {rfc_id}; run `govctl migrate` before RFC lifecycle commands."
            ),
            rfc_id,
        ));
    }
    Err(Diagnostic::new(
        DiagnosticCode::E0102RfcNotFound,
        format!("RFC not found: {rfc_id}"),
        rfc_id,
    ))
}

pub(super) fn require_clause_toml_path(
    config: &Config,
    clause_id: &str,
) -> DiagnosticResult<PathBuf> {
    if let Some(path) = find_clause_toml(config, clause_id) {
        return Ok(path);
    }
    if legacy_clause_json_path(config, clause_id).is_some() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0505MigrationRequired,
            format!(
                "Legacy JSON clause exists for {clause_id}; run `govctl migrate` before clause lifecycle commands."
            ),
            clause_id,
        ));
    }
    Err(Diagnostic::new(
        DiagnosticCode::E0202ClauseNotFound,
        format!("Clause not found: {clause_id}"),
        clause_id,
    ))
}
