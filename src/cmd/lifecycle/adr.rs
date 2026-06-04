use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{AdrEntry, AdrStatus, AlternativeStatus};
use crate::parse::load_adrs;
use crate::parse::write_adr;
use crate::ui;
use crate::validate::is_valid_adr_transition;
use crate::write::WriteOp;

fn adr_not_found(adr_id: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0302AdrNotFound,
        format!("ADR not found: {adr_id}"),
        adr_id,
    )
}

fn replacement_adr_not_found(by: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0302AdrNotFound,
        format!("Replacement ADR not found: {by}"),
        by,
    )
}

fn adr_matches_lifecycle_lookup(entry: &AdrEntry, adr_id: &str) -> bool {
    entry.spec.govctl.id == adr_id || entry.path.to_string_lossy().contains(adr_id)
}

fn load_lifecycle_adr(config: &Config, adr_id: &str) -> DiagnosticResult<AdrEntry> {
    load_adrs(config)?
        .into_iter()
        .find(|entry| adr_matches_lifecycle_lookup(entry, adr_id))
        .ok_or_else(|| adr_not_found(adr_id))
}

fn exact_adr<'a>(adrs: &'a [AdrEntry], adr_id: &str) -> Option<&'a AdrEntry> {
    adrs.iter().find(|entry| entry.spec.govctl.id == adr_id)
}

fn require_replacement_adr(adrs: &[AdrEntry], by: &str) -> DiagnosticResult<()> {
    exact_adr(adrs, by)
        .map(|_| ())
        .ok_or_else(|| replacement_adr_not_found(by))
}

fn take_exact_adr(adrs: Vec<AdrEntry>, adr_id: &str) -> DiagnosticResult<AdrEntry> {
    adrs.into_iter()
        .find(|entry| entry.spec.govctl.id == adr_id)
        .ok_or_else(|| adr_not_found(adr_id))
}

/// Validate ADR alternatives completeness per [[ADR-0042]].
///
/// Requires at least 2 alternatives, with at least 1 accepted and 1 rejected.
pub fn validate_adr_completeness(config: &Config, adr_id: &str) -> DiagnosticResult<()> {
    let entry = load_lifecycle_adr(config, adr_id)?;

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
        ));
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
) -> DiagnosticResult<Diagnostics> {
    let entry = load_lifecycle_adr(config, adr_id)?;

    if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Accepted) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0303AdrInvalidTransition,
            format!(
                "Invalid ADR transition: {} -> accepted",
                entry.spec.govctl.status.as_ref()
            ),
            adr_id,
        ));
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
pub fn reject_adr(config: &Config, adr_id: &str, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    let entry = load_lifecycle_adr(config, adr_id)?;

    if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Rejected) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0303AdrInvalidTransition,
            format!(
                "Invalid ADR transition: {} -> rejected",
                entry.spec.govctl.status.as_ref()
            ),
            adr_id,
        ));
    }

    edit::set_field_direct(config, adr_id, "status", "rejected", op)?;

    if !op.is_preview() {
        ui::rejected("ADR", adr_id);
    }
    Ok(vec![])
}

pub(super) fn supersede_adr(
    config: &Config,
    adr_id: &str,
    by: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let adrs = load_adrs(config)?;

    require_replacement_adr(&adrs, by)?;

    let mut entry = take_exact_adr(adrs, adr_id)?;

    if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Superseded) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0303AdrInvalidTransition,
            format!(
                "Invalid ADR transition: {} -> superseded",
                entry.spec.govctl.status.as_ref()
            ),
            adr_id,
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
        ui::superseded("ADR", adr_id, by);
    }
    Ok(vec![])
}
