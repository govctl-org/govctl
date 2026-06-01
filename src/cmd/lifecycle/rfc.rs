use super::paths::require_rfc_toml_path;
use super::rfc_clause_versions::fill_pending_clause_versions;
use crate::FinalizeStatus;
use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{RfcPhase, RfcStatus};
use crate::ui;
use crate::validate::{is_valid_phase_transition, is_valid_status_transition};
use crate::write::{
    BumpLevel, WriteOp, add_changelog_change, bump_rfc_version, read_rfc, today, write_rfc,
};

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

pub(super) fn supersede_rfc(
    config: &Config,
    rfc_id: &str,
    by: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if rfc_id == by {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "RFC cannot supersede itself",
            rfc_id,
        ));
    }

    let rfc_path = require_rfc_toml_path(config, rfc_id)?;
    let replacement_path = require_rfc_toml_path(config, by).map_err(|err| {
        if err.code == DiagnosticCode::E0102RfcNotFound {
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("Replacement RFC not found: {by}"),
                by,
            )
        } else {
            err
        }
    })?;

    let mut source = read_rfc(config, &rfc_path)?;
    let mut replacement = read_rfc(config, &replacement_path)?;

    if !is_valid_status_transition(source.status, RfcStatus::Deprecated) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Invalid RFC transition: {} -> deprecated",
                source.status.as_ref()
            ),
            rfc_id,
        ));
    }
    if replacement.status == RfcStatus::Deprecated {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!("Replacement RFC is deprecated: {by}"),
            by,
        ));
    }
    if replacement
        .supersedes
        .as_deref()
        .is_some_and(|old| old != rfc_id)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Replacement RFC already supersedes {}",
                replacement.supersedes.as_deref().unwrap_or_default()
            ),
            by,
        ));
    }

    let today = today();
    source.status = RfcStatus::Deprecated;
    source.updated = Some(today.clone());
    replacement.supersedes = Some(rfc_id.to_string());
    replacement.updated = Some(today);
    write_rfc(
        &rfc_path,
        &source,
        op,
        Some(&config.display_path(&rfc_path)),
    )?;
    write_rfc(
        &replacement_path,
        &replacement,
        op,
        Some(&config.display_path(&replacement_path)),
    )?;

    if !op.is_preview() {
        ui::superseded("RFC", rfc_id, by);
    }
    Ok(vec![])
}
