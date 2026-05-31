use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrStatus, AlternativeStatus};
use crate::parse::load_adrs;
use crate::ui;
use crate::validate::is_valid_adr_transition;
use crate::write::WriteOp;

/// Validate ADR alternatives completeness per [[ADR-0042]].
///
/// Requires at least 2 alternatives, with at least 1 accepted and 1 rejected.
pub fn validate_adr_completeness(config: &Config, adr_id: &str) -> anyhow::Result<()> {
    let entry = load_adrs(config)?
        .into_iter()
        .find(|a| a.spec.govctl.id == adr_id || a.path.to_string_lossy().contains(adr_id))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {adr_id}"),
                adr_id,
            )
        })?;

    let alts = &entry.spec.content.alternatives;
    let has_accepted = alts.iter().any(|a| a.status == AlternativeStatus::Accepted);
    let has_rejected = alts.iter().any(|a| a.status == AlternativeStatus::Rejected);

    if alts.len() < 2 || !has_accepted || !has_rejected {
        let mut missing = vec![];
        if alts.len() < 2 {
            missing.push(format!(
                "at least 2 alternatives required (found {})",
                alts.len()
            ));
        }
        if !has_accepted {
            missing.push("at least 1 accepted alternative required".into());
        }
        if !has_rejected {
            missing.push("at least 1 rejected alternative required".into());
        }
        return Err(Diagnostic::new(
            DiagnosticCode::E0303AdrInvalidTransition,
            format!(
                "ADR alternatives incomplete: {}. Use --force to bypass for historical backfills.",
                missing.join("; ")
            ),
            adr_id,
        )
        .into());
    }
    Ok(())
}

/// Accept an ADR
///
/// Per [[ADR-0042]], validates alternatives completeness unless `force` is set.
pub fn accept_adr(
    config: &Config,
    adr_id: &str,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let entry = load_adrs(config)?
        .into_iter()
        .find(|a| a.spec.govctl.id == adr_id || a.path.to_string_lossy().contains(adr_id))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {adr_id}"),
                adr_id,
            )
        })?;

    if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Accepted) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0303AdrInvalidTransition,
            format!(
                "Invalid ADR transition: {} -> accepted",
                entry.spec.govctl.status.as_ref()
            ),
            adr_id,
        )
        .into());
    }

    // Implements [[ADR-0042]]: validate alternatives completeness before acceptance
    if !force {
        validate_adr_completeness(config, adr_id)?;
    }

    edit::set_field_direct(config, adr_id, "status", "accepted", op)?;

    if !op.is_preview() {
        ui::accepted("ADR", adr_id);
    }
    Ok(vec![])
}

/// Reject an ADR
pub fn reject_adr(config: &Config, adr_id: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let entry = load_adrs(config)?
        .into_iter()
        .find(|a| a.spec.govctl.id == adr_id || a.path.to_string_lossy().contains(adr_id))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {adr_id}"),
                adr_id,
            )
        })?;

    if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Rejected) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0303AdrInvalidTransition,
            format!(
                "Invalid ADR transition: {} -> rejected",
                entry.spec.govctl.status.as_ref()
            ),
            adr_id,
        )
        .into());
    }

    edit::set_field_direct(config, adr_id, "status", "rejected", op)?;

    if !op.is_preview() {
        ui::rejected("ADR", adr_id);
    }
    Ok(vec![])
}
