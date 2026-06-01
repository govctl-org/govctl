//! Lifecycle command implementations.

use crate::FinalizeStatus;
use crate::cmd::confirmation::confirm_destructive_action;
use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::find_clause_toml;
use crate::model::{AdrStatus, ClauseStatus};
use crate::parse::{load_adrs, write_adr};
use crate::ui;
use crate::validate::is_valid_adr_transition;
use crate::write::{WriteOp, read_clause, write_clause};
use std::path::PathBuf;

mod adr;
mod release;
mod rfc;
pub use adr::{accept_adr, reject_adr, validate_adr_completeness};
pub use release::cut_release;
pub use rfc::{advance, bump, finalize};

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

/// Deprecate an artifact
///
/// Per [[ADR-0017]], destructive operations require confirmation unless `--force`.
pub fn deprecate(
    config: &Config,
    id: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if !confirm_destructive_action(
        force,
        op,
        &format!("Deprecate {}?", id),
        "Deprecation cancelled",
    )? {
        return Ok(vec![]);
    }

    if id.contains(':') {
        // It's a clause
        let clause_path = require_clause_toml_path(config, id)?;

        let clause = read_clause(config, &clause_path)?;

        if clause.status == ClauseStatus::Deprecated {
            return Err(Diagnostic::new(
                DiagnosticCode::E0208ClauseAlreadyDeprecated,
                "Clause is already deprecated",
                id,
            ));
        }
        if clause.status == ClauseStatus::Superseded {
            return Err(Diagnostic::new(
                DiagnosticCode::E0209ClauseAlreadySuperseded,
                "Clause is superseded, cannot deprecate",
                id,
            ));
        }

        edit::set_field_direct(config, id, "status", "deprecated", op)?;

        if !op.is_preview() {
            ui::deprecated("clause", id);
        }
    } else if id.starts_with("RFC-") {
        // Use finalize for RFC deprecation (confirmation already done above)
        return finalize(config, id, FinalizeStatus::Deprecated, op);
    } else if id.starts_with("ADR-") {
        // ADRs cannot be deprecated; they can only be superseded
        return Err(Diagnostic::new(
            DiagnosticCode::E0305AdrCannotDeprecate,
            format!(
                "ADRs cannot be deprecated. Use `govctl supersede {id} --by ADR-XXXX` instead."
            ),
            id,
        ));
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Unknown artifact type: {id}"),
            id,
        ));
    }

    Ok(vec![])
}

/// Supersede an artifact
///
/// Per [[ADR-0017]], destructive operations require confirmation unless `--force`.
pub fn supersede(
    config: &Config,
    id: &str,
    by: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if !confirm_destructive_action(
        force,
        op,
        &format!("Supersede {} with {}?", id, by),
        "Supersede cancelled",
    )? {
        return Ok(vec![]);
    }

    if id.contains(':') {
        // It's a clause
        // Validate replacement exists
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

        let clause_path = require_clause_toml_path(config, id)?;

        let mut clause = read_clause(config, &clause_path)?;

        if clause.status == ClauseStatus::Superseded {
            return Err(Diagnostic::new(
                DiagnosticCode::E0209ClauseAlreadySuperseded,
                "Clause is already superseded",
                id,
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
            ui::superseded("clause", id, by);
        }
    } else if id.starts_with("ADR-") {
        // Load all ADRs once and find both source and replacement
        let adrs = load_adrs(config)?;

        // Validate replacement exists
        let _ = adrs
            .iter()
            .find(|a| a.spec.govctl.id == by)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0302AdrNotFound,
                    format!("Replacement ADR not found: {by}"),
                    by,
                )
            })?;

        // Find the ADR to supersede
        let mut entry = adrs
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0302AdrNotFound,
                    format!("ADR not found: {id}"),
                    id,
                )
            })?;

        if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Superseded) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0303AdrInvalidTransition,
                format!(
                    "Invalid ADR transition: {} -> superseded",
                    entry.spec.govctl.status.as_ref()
                ),
                id,
            ));
        }

        entry.spec.govctl.status = AdrStatus::Superseded;
        entry.spec.govctl.superseded_by = Some(by.to_string());
        write_adr(
            &entry.path,
            &entry.spec,
            op,
            Some(&config.display_path(&entry.path)),
        )?;

        if !op.is_preview() {
            ui::superseded("ADR", id, by);
        }
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Supersede is not supported for this artifact type: {id}"),
            id,
        ));
    }

    Ok(vec![])
}
