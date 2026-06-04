use super::paths::{require_clause_toml_path, require_replacement_clause_toml_path};
use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::ClauseStatus;
use crate::ui;
use crate::write::{WriteOp, read_clause, write_clause};

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
    require_replacement_clause_toml_path(config, by)?;

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
