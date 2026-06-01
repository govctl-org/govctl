use crate::FinalizeStatus;
use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::find_rfc_toml;
use crate::model::{RfcPhase, RfcStatus};
use crate::ui;
use crate::validate::{is_valid_phase_transition, is_valid_status_transition};
use crate::write::{
    BumpLevel, WriteOp, add_changelog_change, bump_rfc_version, read_clause, read_rfc,
    write_clause, write_rfc,
};
use std::path::{Path, PathBuf};

fn legacy_rfc_json_path(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let path = config.rfc_dir().join(rfc_id).join("rfc.json");
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

/// Update pending clauses (since: null) with the given version.
///
/// Clauses are created with `since: None` and filled in when the RFC
/// is bumped or finalized.
fn fill_pending_clause_versions(
    config: &Config,
    rfc_path: &Path,
    version: &str,
    op: WriteOp,
) -> DiagnosticResult<()> {
    let clauses_dir = rfc_path
        .parent()
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0901IoError,
                "RFC path has no parent directory",
                rfc_path.display().to_string(),
            )
        })?
        .join("clauses");
    if !clauses_dir.exists() {
        return Ok(());
    }

    let mut pending_clauses: Vec<_> = std::fs::read_dir(&clauses_dir)
        .map_err(|err| {
            Diagnostic::io_error(
                "read clauses directory",
                err,
                clauses_dir.display().to_string(),
            )
        })?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .filter_map(|p| read_clause(config, &p).ok().map(|c| (p, c)))
        .filter(|(_, c)| c.since.is_none())
        .collect();

    // Sort by clause_id for deterministic output order
    pending_clauses.sort_by_key(|(_, c)| c.clause_id.clone());

    for (path, mut clause) in pending_clauses {
        clause.since = Some(version.to_string());
        write_clause(&path, &clause, op, Some(&config.display_path(&path)))?;
        if !op.is_preview() {
            ui::sub_info(format!("Set {}.since = {}", clause.clause_id, version));
        }
    }

    Ok(())
}

/// Bump RFC version
pub fn bump(
    config: &Config,
    rfc_id: &str,
    level: Option<BumpLevel>,
    summary: Option<&str>,
    changes: &[String],
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let rfc_path = require_rfc_toml_path(config, rfc_id)?;

    let mut rfc = read_rfc(config, &rfc_path)?;

    match (level, summary, changes.is_empty()) {
        (Some(lvl), Some(sum), _) => {
            let new_version = bump_rfc_version(&mut rfc, lvl, sum)?;
            if !op.is_preview() {
                ui::version_bumped(rfc_id, &new_version);
            }

            for change in changes {
                add_changelog_change(&mut rfc, change)?;
                if !op.is_preview() {
                    ui::sub_info(format!("Added change: {change}"));
                }
            }

            // Write the RFC first
            write_rfc(&rfc_path, &rfc, op, Some(&config.display_path(&rfc_path)))?;

            // Update pending clauses (since: null) with new version
            fill_pending_clause_versions(config, &rfc_path, &new_version, op)?;

            // Then recompute and store signature after version bump per [[ADR-0016]]
            // Load full RFC with clauses to compute accurate signature
            if let Ok(rfc_index) = crate::load::load_rfc(config, &rfc_path)
                && let Ok(sig) = crate::signature::compute_rfc_signature(&rfc_index)
            {
                rfc.signature = Some(sig);
                // Write again with updated signature
                write_rfc(&rfc_path, &rfc, op, Some(&config.display_path(&rfc_path)))?;
            }

            return Ok(vec![]);
        }
        (Some(_), None, _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0108RfcBumpRequiresSummary,
                "--summary is required when bumping version",
                rfc_id,
            ));
        }
        (None, _, false) => {
            for change in changes {
                add_changelog_change(&mut rfc, change)?;
                if !op.is_preview() {
                    ui::changelog_change_added(rfc_id, &rfc.version, change);
                }
            }
        }
        (None, Some(_), true) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0108RfcBumpRequiresSummary,
                "Bump level (--patch/--minor/--major) required when providing --summary",
                rfc_id,
            ));
        }
        (None, None, true) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0801MissingRequiredArg,
                "Provide bump level with --summary, or --change",
                rfc_id,
            ));
        }
    }

    write_rfc(&rfc_path, &rfc, op, Some(&config.display_path(&rfc_path)))?;
    Ok(vec![])
}

/// Finalize RFC status
pub fn finalize(
    config: &Config,
    rfc_id: &str,
    status: FinalizeStatus,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let rfc_path = require_rfc_toml_path(config, rfc_id)?;

    let rfc = read_rfc(config, &rfc_path)?;

    let target_status = match status {
        FinalizeStatus::Normative => RfcStatus::Normative,
        FinalizeStatus::Deprecated => RfcStatus::Deprecated,
    };

    if !is_valid_status_transition(rfc.status, target_status) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Invalid status transition: {} -> {}",
                rfc.status.as_ref(),
                target_status.as_ref()
            ),
            rfc_id,
        ));
    }

    edit::set_field_direct(config, rfc_id, "status", target_status.as_ref(), op)?;

    // Update pending clauses (since: null) with current version
    // When an RFC is finalized, all clauses should have proper since values
    fill_pending_clause_versions(config, &rfc_path, &rfc.version, op)?;

    if !op.is_preview() {
        ui::finalized(rfc_id, target_status.as_ref());
    }
    Ok(vec![])
}

/// Advance RFC phase
pub fn advance(
    config: &Config,
    rfc_id: &str,
    phase: RfcPhase,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let rfc_path = require_rfc_toml_path(config, rfc_id)?;

    let rfc = read_rfc(config, &rfc_path)?;

    // Check status constraint: cannot advance to impl+ without normative status
    if rfc.status == RfcStatus::Draft && phase != RfcPhase::Spec {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot advance to {} while status is draft. Finalize to normative first.",
                phase.as_ref()
            ),
            rfc_id,
        ));
    }

    if !is_valid_phase_transition(rfc.phase, phase) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Invalid phase transition: {} -> {}",
                rfc.phase.as_ref(),
                phase.as_ref()
            ),
            rfc_id,
        ));
    }

    edit::set_field_direct(config, rfc_id, "phase", phase.as_ref(), op)?;

    if !op.is_preview() {
        ui::phase_advanced(rfc_id, phase.as_ref());
    }
    Ok(vec![])
}
