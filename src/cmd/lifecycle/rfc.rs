use super::paths::require_rfc_toml_path;
use super::rfc_clause_versions::fill_pending_clause_versions;
use crate::FinalizeStatus;
use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{RfcPhase, RfcSpec, RfcStatus};
use crate::ui;
use crate::validate::{is_valid_phase_transition, is_valid_status_transition};
use crate::write::{BumpLevel, WriteOp, add_changelog_change, bump_rfc_version, read_rfc};
use std::path::Path;

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
    let refresh_signature_after_write = match (level, summary, changes.is_empty()) {
        (Some(lvl), Some(sum), _) => {
            ensure_rfc_has_content_amendment(config, &rfc_path, rfc_id)?;

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

            write_lifecycle_rfc(config, &rfc_path, &rfc, op)?;

            fill_pending_clause_versions(config, &rfc_path, &new_version, op)?;
            refresh_rfc_signature_best_effort(config, &rfc_path, &mut rfc, op)?;

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
            let refresh_signature_after_write =
                should_refresh_signature_after_changelog_only(config, &rfc_path)?;
            for change in changes {
                add_changelog_change(&mut rfc, change)?;
                if !op.is_preview() {
                    ui::changelog_change_added(rfc_id, &rfc.version, change);
                }
            }
            refresh_signature_after_write
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
    };

    write_lifecycle_rfc(config, &rfc_path, &rfc, op)?;
    if refresh_signature_after_write {
        refresh_rfc_signature_best_effort(config, &rfc_path, &mut rfc, op)?;
    }
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

fn write_lifecycle_rfc(
    config: &Config,
    rfc_path: &Path,
    rfc: &RfcSpec,
    op: WriteOp,
) -> DiagnosticResult<()> {
    crate::write::write_rfc(rfc_path, rfc, op, Some(&config.display_path(rfc_path)))
}

fn refresh_rfc_signature_best_effort(
    config: &Config,
    rfc_path: &Path,
    rfc: &mut RfcSpec,
    op: WriteOp,
) -> DiagnosticResult<()> {
    if let Ok(rfc_index) = crate::load::load_rfc(config, rfc_path)
        && let Ok(sig) = crate::signature::compute_rfc_content_signature(&rfc_index)
    {
        rfc.signature = Some(sig);
        write_lifecycle_rfc(config, rfc_path, rfc, op)?;
    }
    Ok(())
}

fn ensure_rfc_has_content_amendment(
    config: &Config,
    rfc_path: &Path,
    rfc_id: &str,
) -> DiagnosticResult<()> {
    let rfc_index = crate::load::load_rfc(config, rfc_path)?;
    if rfc_index.rfc.signature.is_none() || crate::signature::is_rfc_amended(&rfc_index) {
        return Ok(());
    }

    Err(Diagnostic::new(
        DiagnosticCode::E0113RfcBumpNoAmendment,
        "RFC version bump requires RFC or clause content changes since the last bump",
        rfc_id,
    ))
}

fn should_refresh_signature_after_changelog_only(
    config: &Config,
    rfc_path: &Path,
) -> DiagnosticResult<bool> {
    let rfc_index = crate::load::load_rfc(config, rfc_path)?;
    Ok(rfc_index.rfc.signature.is_some() && !crate::signature::is_rfc_amended(&rfc_index))
}
