use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::find_clause_toml;
use crate::model::ClauseStatus;
use crate::ui;
use crate::write::{WriteOp, read_clause, write_clause};
use std::path::PathBuf;

fn legacy_clause_json_path(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let (rfc_id, clause_name) = clause_id.split_once(':')?;
    let path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.json"));
    path.exists().then_some(path)
}

fn require_clause_toml_path(config: &Config, clause_id: &str) -> DiagnosticResult<PathBuf> {
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

pub(super) fn deprecate_clause(
    config: &Config,
    clause_id: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let clause_path = require_clause_toml_path(config, clause_id)?;
    let clause = read_clause(config, &clause_path)?;

    if clause.status == ClauseStatus::Deprecated {
        return Err(Diagnostic::new(
            DiagnosticCode::E0208ClauseAlreadyDeprecated,
            "Clause is already deprecated",
            clause_id,
        ));
    }
    if clause.status == ClauseStatus::Superseded {
        return Err(Diagnostic::new(
            DiagnosticCode::E0209ClauseAlreadySuperseded,
            "Clause is superseded, cannot deprecate",
            clause_id,
        ));
    }

    edit::set_field_direct(config, clause_id, "status", "deprecated", op)?;

    if !op.is_preview() {
        ui::deprecated("clause", clause_id);
    }
    Ok(vec![])
}

pub(super) fn supersede_clause(
    config: &Config,
    clause_id: &str,
    by: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if let Err(err) = require_clause_toml_path(config, by) {
        if err.code == DiagnosticCode::E0202ClauseNotFound {
            return Err(Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Replacement clause not found: {by}"),
                by,
            ));
        }
        return Err(err);
    }

    let clause_path = require_clause_toml_path(config, clause_id)?;
    let mut clause = read_clause(config, &clause_path)?;

    if clause.status == ClauseStatus::Superseded {
        return Err(Diagnostic::new(
            DiagnosticCode::E0209ClauseAlreadySuperseded,
            "Clause is already superseded",
            clause_id,
        ));
    }

    clause.status = ClauseStatus::Superseded;
    clause.superseded_by = Some(by.to_string());
    write_clause(
        &clause_path,
        &clause,
        op,
        Some(&config.display_path(&clause_path)),
    )?;

    if !op.is_preview() {
        ui::superseded("clause", clause_id, by);
    }
    Ok(vec![])
}
