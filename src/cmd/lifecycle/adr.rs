use crate::cmd::edit;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{AdrEntry, AdrStatus, AlternativeStatus};
use crate::parse::load_adrs;
use crate::parse::write_adr;
use crate::ui;
use crate::validate::{is_valid_adr_transition, validate_adr_projection};
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
    if adr_id.starts_with("ADR-") {
        // [[RFC-0002:C-LIFECYCLE-VERBS]]: ADR lifecycle verbs operate on
        // existing ADR IDs; normalize catalog misses to the ADR not-found path.
        return crate::artifact_catalog::load_adr_by_id(config, adr_id).map_err(|err| {
            if err.code == DiagnosticCode::E0302AdrNotFound {
                adr_not_found(adr_id)
            } else {
                err
            }
        });
    }

    load_adrs(config)?
        .into_iter()
        .find(|entry| adr_matches_lifecycle_lookup(entry, adr_id))
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
                "Invalid ADR transition: {} -> accepted. Valid transitions from {}: {}",
                entry.spec.govctl.status.as_ref(),
                entry.spec.govctl.status.as_ref(),
                valid_adr_targets(entry.spec.govctl.status)
            ),
            adr_id,
        ));
    }

    if let Some(diagnostic) = validate_adr_projection(
        &entry,
        config.display_path(&entry.path).display().to_string(),
    )
    .into_iter()
    .next()
    {
        return Err(diagnostic);
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
                "Invalid ADR transition: {} -> rejected. Valid transitions from {}: {}",
                entry.spec.govctl.status.as_ref(),
                entry.spec.govctl.status.as_ref(),
                valid_adr_targets(entry.spec.govctl.status)
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
    // [[RFC-0002:C-LIFECYCLE-VERBS]] and [[RFC-0001:C-ADR-STATUS]]: ADR
    // supersede requires `--by` to identify an existing replacement ADR.
    match crate::artifact_catalog::load_adr_by_id(config, by) {
        Ok(_) => {}
        Err(err) if err.code == DiagnosticCode::E0302AdrNotFound => {
            return Err(replacement_adr_not_found(by));
        }
        Err(err) => return Err(err),
    }
    let mut entry = load_lifecycle_adr(config, adr_id)?;

    if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Superseded) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0303AdrInvalidTransition,
            format!(
                "Invalid ADR transition: {} -> superseded. Valid transitions from {}: {}",
                entry.spec.govctl.status.as_ref(),
                entry.spec.govctl.status.as_ref(),
                valid_adr_targets(entry.spec.govctl.status)
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

fn valid_adr_targets(status: AdrStatus) -> &'static str {
    match status {
        AdrStatus::Proposed => "accepted, rejected",
        AdrStatus::Accepted => "superseded",
        AdrStatus::Rejected => "none (rejected is terminal)",
        AdrStatus::Superseded => "none (superseded is terminal)",
    }
}
