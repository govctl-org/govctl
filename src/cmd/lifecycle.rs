//! Lifecycle command implementations.

use crate::FinalizeStatus;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::{find_clause_json, find_rfc_json};
use crate::model::{AdrStatus, ClauseStatus, RfcPhase, RfcStatus};
use crate::parse::{load_adrs, write_adr};
use crate::ui;
use crate::validate::{
    is_valid_adr_transition, is_valid_phase_transition, is_valid_status_transition,
};
use crate::write::{
    BumpLevel, add_changelog_change, bump_rfc_version, read_clause, read_rfc, write_clause,
    write_rfc,
};

/// Bump RFC version
pub fn bump(
    config: &Config,
    rfc_id: &str,
    level: Option<BumpLevel>,
    summary: Option<&str>,
    changes: &[String],
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path =
        find_rfc_json(config, rfc_id).ok_or_else(|| anyhow::anyhow!("RFC not found: {rfc_id}"))?;

    let mut rfc = read_rfc(&rfc_path)?;

    match (level, summary, changes.is_empty()) {
        (Some(lvl), Some(sum), _) => {
            let new_version = bump_rfc_version(&mut rfc, lvl, sum)?;
            ui::version_bumped(rfc_id, &new_version);

            for change in changes {
                add_changelog_change(&mut rfc, change)?;
                ui::sub_info(format!("Added change: {change}"));
            }
        }
        (Some(_), None, _) => {
            anyhow::bail!("--summary is required when bumping version");
        }
        (None, _, false) => {
            for change in changes {
                add_changelog_change(&mut rfc, change)?;
                ui::changelog_change_added(rfc_id, &rfc.version, change);
            }
        }
        (None, Some(_), true) => {
            anyhow::bail!("Bump level (--patch/--minor/--major) required when providing --summary");
        }
        (None, None, true) => {
            anyhow::bail!("Provide bump level with --summary, or --change");
        }
    }

    write_rfc(&rfc_path, &rfc)?;
    Ok(vec![])
}

/// Finalize RFC status
pub fn finalize(
    config: &Config,
    rfc_id: &str,
    status: FinalizeStatus,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path =
        find_rfc_json(config, rfc_id).ok_or_else(|| anyhow::anyhow!("RFC not found: {rfc_id}"))?;

    let mut rfc = read_rfc(&rfc_path)?;

    let target_status = match status {
        FinalizeStatus::Normative => RfcStatus::Normative,
        FinalizeStatus::Deprecated => RfcStatus::Deprecated,
    };

    if !is_valid_status_transition(rfc.status, target_status) {
        anyhow::bail!(
            "Invalid status transition: {} -> {}",
            rfc.status.as_ref(),
            target_status.as_ref()
        );
    }

    rfc.status = target_status;
    rfc.updated = Some(crate::write::today());
    write_rfc(&rfc_path, &rfc)?;

    ui::finalized(rfc_id, target_status.as_ref());
    Ok(vec![])
}

/// Advance RFC phase
pub fn advance(config: &Config, rfc_id: &str, phase: RfcPhase) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path =
        find_rfc_json(config, rfc_id).ok_or_else(|| anyhow::anyhow!("RFC not found: {rfc_id}"))?;

    let mut rfc = read_rfc(&rfc_path)?;

    // Check status constraint: cannot advance to impl+ without normative status
    if rfc.status == RfcStatus::Draft && phase != RfcPhase::Spec {
        anyhow::bail!(
            "Cannot advance to {} while status is draft. Finalize to normative first.",
            phase.as_ref()
        );
    }

    if !is_valid_phase_transition(rfc.phase, phase) {
        anyhow::bail!(
            "Invalid phase transition: {} -> {}",
            rfc.phase.as_ref(),
            phase.as_ref()
        );
    }

    rfc.phase = phase;
    rfc.updated = Some(crate::write::today());
    write_rfc(&rfc_path, &rfc)?;

    ui::phase_advanced(rfc_id, phase.as_ref());
    Ok(vec![])
}

/// Accept an ADR
pub fn accept_adr(config: &Config, adr_id: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let mut entry = load_adrs(config)?
        .into_iter()
        .find(|a| a.spec.govctl.id == adr_id || a.path.to_string_lossy().contains(adr_id))
        .ok_or_else(|| anyhow::anyhow!("ADR not found: {adr_id}"))?;

    if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Accepted) {
        anyhow::bail!(
            "Invalid ADR transition: {} -> accepted",
            entry.spec.govctl.status.as_ref()
        );
    }

    entry.spec.govctl.status = AdrStatus::Accepted;
    write_adr(&entry.path, &entry.spec)?;

    ui::accepted("ADR", adr_id);
    Ok(vec![])
}

/// Deprecate an artifact
pub fn deprecate(config: &Config, id: &str) -> anyhow::Result<Vec<Diagnostic>> {
    if id.contains(':') {
        // It's a clause
        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        let mut clause = read_clause(&clause_path)?;

        if clause.status == ClauseStatus::Deprecated {
            anyhow::bail!("Clause is already deprecated");
        }
        if clause.status == ClauseStatus::Superseded {
            anyhow::bail!("Clause is superseded, cannot deprecate");
        }

        clause.status = ClauseStatus::Deprecated;
        write_clause(&clause_path, &clause)?;

        ui::deprecated("clause", id);
    } else if id.starts_with("RFC-") {
        // Use finalize for RFC deprecation
        return finalize(config, id, FinalizeStatus::Deprecated);
    } else if id.starts_with("ADR-") {
        // ADRs cannot be deprecated; they can only be superseded
        anyhow::bail!(
            "ADRs cannot be deprecated. Use `govctl supersede {id} --by ADR-XXXX` instead."
        );
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}

/// Supersede an artifact
pub fn supersede(config: &Config, id: &str, by: &str) -> anyhow::Result<Vec<Diagnostic>> {
    if id.contains(':') {
        // It's a clause
        // Validate replacement exists
        let _ = find_clause_json(config, by)
            .ok_or_else(|| anyhow::anyhow!("Replacement clause not found: {by}"))?;

        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        let mut clause = read_clause(&clause_path)?;

        if clause.status == ClauseStatus::Superseded {
            anyhow::bail!("Clause is already superseded");
        }

        clause.status = ClauseStatus::Superseded;
        clause.superseded_by = Some(by.to_string());
        write_clause(&clause_path, &clause)?;

        ui::superseded("clause", id, by);
    } else if id.starts_with("ADR-") {
        // Load all ADRs once and find both source and replacement
        let adrs = load_adrs(config)?;

        // Validate replacement exists
        let _ = adrs
            .iter()
            .find(|a| a.spec.govctl.id == by)
            .ok_or_else(|| anyhow::anyhow!("Replacement ADR not found: {by}"))?;

        // Find the ADR to supersede
        let mut entry = adrs
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Superseded) {
            anyhow::bail!(
                "Invalid ADR transition: {} -> superseded",
                entry.spec.govctl.status.as_ref()
            );
        }

        entry.spec.govctl.status = AdrStatus::Superseded;
        entry.spec.govctl.superseded_by = Some(by.to_string());
        write_adr(&entry.path, &entry.spec)?;

        ui::superseded("ADR", id, by);
    } else {
        anyhow::bail!("Supersede is not supported for this artifact type: {id}");
    }

    Ok(vec![])
}
