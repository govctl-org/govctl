use super::ValidationResult;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseStatus, ProjectIndex, RfcIndex, RfcPhase, RfcStatus};
use std::collections::HashSet;

pub(super) fn validate_rfc(rfc: &RfcIndex, config: &Config, result: &mut ValidationResult) {
    let rfc_path_display = config.display_path(&rfc.path).display().to_string();

    let dir_name = rfc
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    if let Some(name) = dir_name
        && name != rfc.rfc.rfc_id
    {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0103RfcIdMismatch,
            format!(
                "RFC ID '{}' doesn't match directory '{}'",
                rfc.rfc.rfc_id, name
            ),
            rfc_path_display.clone(),
        ));
    }

    if rfc.rfc.changelog.is_empty() {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::W0101RfcNoChangelog,
            "RFC has no changelog entries (hint: run `govctl rfc bump`)",
            rfc_path_display.clone(),
        ));
    }

    validate_status_phase_constraints(rfc, config, result);

    for clause in &rfc.clauses {
        let clause_path_display = config.display_path(&clause.path).display().to_string();
        if clause.spec.since.is_none() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0102ClauseNoSince,
                format!(
                    "Clause '{}' has no 'since' version (hint: it will be set automatically by `govctl rfc bump` or `govctl rfc finalize`)",
                    clause.spec.clause_id
                ),
                clause_path_display.clone(),
            ));
        }

        let file_name = clause
            .path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name != clause.spec.clause_id {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0203ClauseIdMismatch,
                format!(
                    "Clause ID '{}' doesn't match filename '{}'",
                    clause.spec.clause_id, file_name
                ),
                clause_path_display,
            ));
        }
    }
}

fn validate_status_phase_constraints(
    rfc: &RfcIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let status = rfc.rfc.status;
    let phase = rfc.rfc.phase;
    let path_display = config.display_path(&rfc.path).display().to_string();

    if status == RfcStatus::Draft && phase == RfcPhase::Stable {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            "Cannot have status=draft with phase=stable",
            path_display.clone(),
        ));
    }

    if status == RfcStatus::Deprecated && (phase == RfcPhase::Impl || phase == RfcPhase::Test) {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot have status=deprecated with phase={}",
                phase.as_ref()
            ),
            path_display,
        ));
    }
}

pub(super) fn validate_clause_references(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let active_clauses: HashSet<String> = index
        .iter_clauses()
        .filter(|(_, c)| c.spec.status == ClauseStatus::Active)
        .map(|(rfc, c)| format!("{}:{}", rfc.rfc.rfc_id, c.spec.clause_id))
        .collect();

    for (rfc, clause) in index.iter_clauses() {
        if let Some(ref superseded_by) = clause.spec.superseded_by {
            let clause_path_display = config.display_path(&clause.path).display().to_string();
            if clause.spec.status != ClauseStatus::Superseded {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0206ClauseSupersededByUnknown,
                    format!(
                        "Clause has superseded_by but status is not 'superseded': {}",
                        clause.spec.clause_id
                    ),
                    clause_path_display.clone(),
                ));
            }

            let full_ref = if superseded_by.contains(':') {
                superseded_by.clone()
            } else {
                format!("{}:{}", rfc.rfc.rfc_id, superseded_by)
            };

            if !active_clauses.contains(&full_ref) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0207ClauseSupersededByNotActive,
                    format!(
                        "Clause '{}' superseded by '{}' which is not active",
                        clause.spec.clause_id, superseded_by
                    ),
                    clause_path_display,
                ));
            }
        }
    }
}
