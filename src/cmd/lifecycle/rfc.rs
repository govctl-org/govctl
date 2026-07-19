use super::paths::require_rfc_toml_path;
use super::rfc_clause_versions::{
    fill_pending_clause_versions, pending_clause_ids, rfc_update_paths,
};
use crate::FinalizeStatus;
use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{RfcPhase, RfcSpec, RfcStatus};
use crate::ui;
use crate::validate::{is_valid_phase_transition, is_valid_status_transition};
use crate::write::{
    BumpLevel, WriteOp, add_changelog_change, bump_rfc_version, read_rfc, with_file_transaction,
};
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
    require_rfc_content_signature_schema(config, rfc_id)?;
    let rfc_path = require_rfc_toml_path(config, rfc_id)?;

    let mut rfc = read_rfc(config, &rfc_path)?;
    match (level, summary, changes.is_empty()) {
        (Some(lvl), Some(sum), _) => {
            // [[RFC-0002:C-LIFECYCLE-VERBS]] reserves version-changing bumps for
            // RFC lineages that have crossed the normative publication boundary.
            if rfc.status != RfcStatus::Normative {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0104RfcInvalidTransition,
                    format!(
                        "Cannot bump RFC version while status={}. Version-changing bumps require normative RFC status.",
                        rfc.status.as_ref()
                    ),
                    rfc_id,
                ));
            }

            // [[RFC-0000:C-PHASE-LIFECYCLE]] permits a version-changing bump only
            // after the current version has been sealed with a stored signature.
            if rfc.phase == RfcPhase::Spec {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0104RfcInvalidTransition,
                    "Cannot bump RFC version while phase=spec. Continue authoring the current version candidate, then advance it to impl before opening another version.",
                    rfc_id,
                ));
            }

            if rfc.signature.is_none() {
                return Err(missing_sealed_signature(rfc_id, "bump RFC version"));
            }

            ensure_rfc_has_content_amendment(config, &rfc_path, rfc_id)?;

            let new_version = bump_rfc_version(&mut rfc, lvl, sum)?;
            rfc.phase = RfcPhase::Spec;

            for change in changes {
                add_changelog_change(&mut rfc, change)?;
            }

            let paths = rfc_update_paths(config, &rfc_path)?;
            let path_refs: Vec<_> = paths.iter().map(std::path::PathBuf::as_path).collect();
            let updated_clause_ids = with_file_transaction(&path_refs, op, || {
                write_lifecycle_rfc(config, &rfc_path, &rfc, op)?;
                let updated_clause_ids =
                    fill_pending_clause_versions(config, &rfc_path, &new_version, op)?;
                Ok(updated_clause_ids)
            })?;

            if !op.is_preview() {
                ui::version_bumped(rfc_id, &new_version);
                for change in changes {
                    ui::sub_info(format!("Added change: {change}"));
                }
                for clause_id in updated_clause_ids {
                    ui::sub_info(format!("Set {clause_id}.since = {new_version}"));
                }
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
        (None, Some(_), _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0108RfcBumpRequiresSummary,
                "Bump level (--patch/--minor/--major) required when providing --summary",
                rfc_id,
            ));
        }
        (None, None, false) => {
            require_changelog_update_ready(config, &rfc_path, rfc_id)?;
            for change in changes {
                add_changelog_change(&mut rfc, change)?;
            }
        }
        (None, None, true) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0801MissingRequiredArg,
                "Provide bump level with --summary, or --change",
                rfc_id,
            ));
        }
    }

    with_file_transaction(&[rfc_path.as_path()], op, || {
        write_lifecycle_rfc(config, &rfc_path, &rfc, op)
    })?;
    if !op.is_preview() {
        for change in changes {
            ui::changelog_change_added(rfc_id, &rfc.version, change);
        }
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
    match status {
        FinalizeStatus::Normative => {
            transition_rfc_status(config, rfc_id, RfcStatus::Normative, op)
        }
    }
}

pub(super) fn deprecate_rfc(
    config: &Config,
    rfc_id: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    transition_rfc_status(config, rfc_id, RfcStatus::Deprecated, op)
}

fn transition_rfc_status(
    config: &Config,
    rfc_id: &str,
    target_status: RfcStatus,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let rfc_path = require_rfc_toml_path(config, rfc_id)?;

    let rfc = read_rfc(config, &rfc_path)?;

    if !is_valid_status_transition(rfc.status, target_status) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Invalid status transition: {} -> {}. Valid transition from {}: {}",
                rfc.status.as_ref(),
                target_status.as_ref(),
                rfc.status.as_ref(),
                valid_rfc_status_targets(rfc.status)
            ),
            rfc_id,
        ));
    }

    if target_status == RfcStatus::Deprecated
        && matches!(rfc.phase, RfcPhase::Impl | RfcPhase::Test)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot deprecate an RFC while phase is {}. Advance the current version to stable first.",
                rfc.phase.as_ref()
            ),
            rfc_id,
        ));
    }

    let updated_clause_ids = if target_status == RfcStatus::Normative {
        let paths = rfc_update_paths(config, &rfc_path)?;
        let path_refs: Vec<_> = paths.iter().map(std::path::PathBuf::as_path).collect();
        with_file_transaction(&path_refs, op, || {
            edit::set_field_direct(config, rfc_id, "status", target_status.as_ref(), op)?;
            fill_pending_clause_versions(config, &rfc_path, &rfc.version, op)
        })?
    } else {
        edit::set_field_direct(config, rfc_id, "status", target_status.as_ref(), op)?;
        Vec::new()
    };

    if !op.is_preview() {
        for clause_id in updated_clause_ids {
            ui::sub_info(format!("Set {clause_id}.since = {}", rfc.version));
        }
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
    require_rfc_content_signature_schema(config, rfc_id)?;
    let rfc_path = require_rfc_toml_path(config, rfc_id)?;

    let rfc = read_rfc(config, &rfc_path)?;

    // Phase/status combinations are constrained by [[RFC-0000:C-PHASE-LIFECYCLE]].
    if rfc.status != RfcStatus::Normative && phase != RfcPhase::Spec {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot advance to {} while status is {}. Only normative RFCs can enter implementation phases.",
                phase.as_ref(),
                rfc.status.as_ref()
            ),
            rfc_id,
        ));
    }

    if !is_valid_phase_transition(rfc.phase, phase) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Invalid phase transition: {} -> {}. Valid transition from {}: {}",
                rfc.phase.as_ref(),
                phase.as_ref(),
                rfc.phase.as_ref(),
                valid_rfc_phase_targets(rfc.phase)
            ),
            rfc_id,
        ));
    }

    let seals_current_version = rfc.phase == RfcPhase::Spec && phase == RfcPhase::Impl;
    if seals_current_version {
        let pending_clause_ids = pending_clause_ids(config, &rfc_path)?;
        if !pending_clause_ids.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0104RfcInvalidTransition,
                format!(
                    "Cannot advance to impl while Clause versions are pending: {}",
                    pending_clause_ids.join(", ")
                ),
                rfc_id,
            ));
        }
    }

    // [[RFC-0000:C-PHASE-LIFECYCLE]] requires every post-spec phase to retain
    // the sealed signature established by the spec -> impl transition.
    if !seals_current_version && rfc.signature.is_none() {
        return Err(missing_sealed_signature(rfc_id, "advance RFC phase"));
    }

    let rfc_index = crate::load::load_rfc(config, &rfc_path)?;
    let current_signature = crate::signature::compute_rfc_content_signature(&rfc_index)?;
    let next_signature = if seals_current_version {
        Some(current_signature)
    } else {
        let stored_signature = rfc_index
            .rfc
            .signature
            .as_ref()
            .ok_or_else(|| missing_sealed_signature(rfc_id, "advance RFC phase"))?;
        if stored_signature == &current_signature {
            None
        } else if crate::signature::compute_rfc_signature(&rfc_index)
            .is_ok_and(|legacy| stored_signature == &legacy)
        {
            return Err(Diagnostic::new(
                DiagnosticCode::E0505MigrationRequired,
                "Cannot advance phase while the RFC has a legacy amendment signature. Run `govctl migrate` before advancing.",
                rfc_id,
            ));
        } else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0114RfcPendingAmendment,
                "Cannot advance phase while RFC or clause content has an unversioned amendment. Release it with `govctl rfc bump` first.",
                rfc_id,
            ));
        }
    };

    let mut updated_rfc = rfc;
    if let Some(signature) = next_signature {
        updated_rfc.signature = Some(signature);
    }
    updated_rfc.phase = phase;
    write_lifecycle_rfc(config, &rfc_path, &updated_rfc, op)?;

    if !op.is_preview() {
        ui::phase_advanced(rfc_id, phase.as_ref());
    }
    Ok(vec![])
}

fn valid_rfc_status_targets(status: RfcStatus) -> &'static str {
    match status {
        RfcStatus::Draft => "normative",
        RfcStatus::Normative => "deprecated",
        RfcStatus::Deprecated => "none (deprecated is terminal)",
    }
}

fn valid_rfc_phase_targets(phase: RfcPhase) -> &'static str {
    match phase {
        RfcPhase::Spec => "impl",
        RfcPhase::Impl => "test",
        RfcPhase::Test => "stable",
        RfcPhase::Stable => "none (stable is terminal for the current version)",
    }
}

fn require_rfc_content_signature_schema(config: &Config, rfc_id: &str) -> DiagnosticResult<()> {
    // Older schemas require the explicit migration path in [[RFC-0002:C-GLOBAL-COMMANDS]].
    let required = crate::cmd::migrate::RFC_CONTENT_SIGNATURE_SCHEMA_VERSION;
    if config.schema.version >= required {
        return Ok(());
    }

    Err(Diagnostic::new(
        DiagnosticCode::E0505MigrationRequired,
        format!(
            "RFC amendment signatures require schema version {required} (found {}). Run `govctl migrate` before bumping or advancing RFCs.",
            config.schema.version
        ),
        rfc_id,
    ))
}

fn write_lifecycle_rfc(
    config: &Config,
    rfc_path: &Path,
    rfc: &RfcSpec,
    op: WriteOp,
) -> DiagnosticResult<()> {
    crate::write::write_rfc(rfc_path, rfc, op, Some(&config.display_path(rfc_path)))
}

fn ensure_rfc_has_content_amendment(
    config: &Config,
    rfc_path: &Path,
    rfc_id: &str,
) -> DiagnosticResult<()> {
    if !pending_clause_ids(config, rfc_path)?.is_empty() {
        return Ok(());
    }

    let rfc_index = crate::load::load_rfc(config, rfc_path)?;
    if rfc_index.rfc.signature.is_none() {
        return Err(missing_sealed_signature(rfc_id, "bump RFC version"));
    }
    if crate::signature::is_rfc_amended(&rfc_index) {
        return Ok(());
    }

    Err(Diagnostic::new(
        DiagnosticCode::E0113RfcBumpNoAmendment,
        "RFC version bump requires RFC or clause content changes since the last bump",
        rfc_id,
    ))
}

fn missing_sealed_signature(rfc_id: &str, action: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0505MigrationRequired,
        format!(
            "Cannot {action} without a sealed RFC content signature. Run `govctl migrate` or restore the sealed signature baseline from version-control history."
        ),
        rfc_id,
    )
}

pub(crate) fn require_changelog_update_ready(
    config: &Config,
    rfc_path: &Path,
    rfc_id: &str,
) -> DiagnosticResult<()> {
    require_rfc_content_signature_schema(config, rfc_id)?;
    let rfc_index = crate::load::load_rfc(config, rfc_path)?;
    let Some(stored_signature) = &rfc_index.rfc.signature else {
        return Ok(());
    };
    let content_signature = crate::signature::compute_rfc_content_signature(&rfc_index)?;
    if stored_signature == &content_signature {
        return Ok(());
    }
    if crate::signature::compute_rfc_signature(&rfc_index)
        .is_ok_and(|legacy| stored_signature == &legacy)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0505MigrationRequired,
            "Cannot update the changelog while the RFC has a legacy amendment signature. Migrate the repository signature baseline first.",
            rfc_id,
        ));
    }
    Ok(())
}
