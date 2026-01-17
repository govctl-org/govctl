//! Lifecycle command implementations.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::{find_clause_json, find_rfc_json};
use crate::model::{
    AdrStatus, ClauseStatus, PhaseOsWrapper, RfcPhase, RfcStatus,
};
use crate::validate::{is_valid_adr_transition, is_valid_phase_transition, is_valid_status_transition};
use crate::write::{
    add_changelog_change, bump_rfc_version, read_clause, read_rfc, write_clause, write_rfc,
    BumpLevel,
};
use crate::FinalizeStatus;

/// Bump RFC version
pub fn bump(
    config: &Config,
    rfc_id: &str,
    level: Option<BumpLevel>,
    summary: Option<&str>,
    changes: &[String],
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path = find_rfc_json(config, rfc_id)
        .ok_or_else(|| anyhow::anyhow!("RFC not found: {rfc_id}"))?;

    let mut rfc = read_rfc(&rfc_path)?;

    match (level, summary, changes.is_empty()) {
        (Some(lvl), Some(sum), _) => {
            let new_version = bump_rfc_version(&mut rfc, lvl, sum)?;
            eprintln!("Bumped {rfc_id} to {new_version}");

            for change in changes {
                add_changelog_change(&mut rfc, change)?;
                eprintln!("  Added change: {change}");
            }
        }
        (Some(_), None, _) => {
            anyhow::bail!("--summary is required when bumping version");
        }
        (None, _, false) => {
            for change in changes {
                add_changelog_change(&mut rfc, change)?;
                eprintln!("Added change to {rfc_id} v{}: {change}", rfc.version);
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
    let rfc_path = find_rfc_json(config, rfc_id)
        .ok_or_else(|| anyhow::anyhow!("RFC not found: {rfc_id}"))?;

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

    eprintln!("Finalized {rfc_id} to status: {}", target_status.as_ref());
    Ok(vec![])
}

/// Advance RFC phase
pub fn advance(
    config: &Config,
    rfc_id: &str,
    phase: RfcPhase,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path = find_rfc_json(config, rfc_id)
        .ok_or_else(|| anyhow::anyhow!("RFC not found: {rfc_id}"))?;

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

    eprintln!("Advanced {rfc_id} to phase: {}", phase.as_ref());
    Ok(vec![])
}

/// Accept an ADR
pub fn accept_adr(config: &Config, adr_id: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let adr = crate::parse::load_adrs(config)?
        .into_iter()
        .find(|a| a.meta.id == adr_id || a.path.to_string_lossy().contains(adr_id))
        .ok_or_else(|| anyhow::anyhow!("ADR not found: {adr_id}"))?;

    if !is_valid_adr_transition(adr.meta.status, AdrStatus::Accepted) {
        anyhow::bail!(
            "Invalid ADR transition: {} -> accepted",
            adr.meta.status.as_ref()
        );
    }

    let mut meta = adr.meta;
    meta.status = AdrStatus::Accepted;

    let wrapper = PhaseOsWrapper {
        phaseos: meta,
        ext: None,
    };
    crate::parse::update_frontmatter(&adr.path, &wrapper)?;

    eprintln!("Accepted ADR: {adr_id}");
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

        eprintln!("Deprecated clause: {id}");
    } else if id.starts_with("RFC-") {
        // Use finalize for RFC deprecation
        return finalize(config, id, FinalizeStatus::Deprecated);
    } else if id.starts_with("ADR-") {
        let adr = crate::parse::load_adrs(config)?
            .into_iter()
            .find(|a| a.meta.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        if !is_valid_adr_transition(adr.meta.status, AdrStatus::Deprecated) {
            anyhow::bail!(
                "Invalid ADR transition: {} -> deprecated",
                adr.meta.status.as_ref()
            );
        }

        let mut meta = adr.meta;
        meta.status = AdrStatus::Deprecated;

        let wrapper = PhaseOsWrapper {
            phaseos: meta,
            ext: None,
        };
        crate::parse::update_frontmatter(&adr.path, &wrapper)?;

        eprintln!("Deprecated ADR: {id}");
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

        eprintln!("Superseded clause: {id}");
        eprintln!("  Replaced by: {by}");
    } else if id.starts_with("ADR-") {
        // Validate replacement exists
        let _ = crate::parse::load_adrs(config)?
            .into_iter()
            .find(|a| a.meta.id == by)
            .ok_or_else(|| anyhow::anyhow!("Replacement ADR not found: {by}"))?;

        let adr = crate::parse::load_adrs(config)?
            .into_iter()
            .find(|a| a.meta.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        if !is_valid_adr_transition(adr.meta.status, AdrStatus::Superseded) {
            anyhow::bail!(
                "Invalid ADR transition: {} -> superseded",
                adr.meta.status.as_ref()
            );
        }

        let mut meta = adr.meta;
        meta.status = AdrStatus::Superseded;
        meta.superseded_by = Some(by.to_string());

        let wrapper = PhaseOsWrapper {
            phaseos: meta,
            ext: None,
        };
        crate::parse::update_frontmatter(&adr.path, &wrapper)?;

        eprintln!("Superseded ADR: {id}");
        eprintln!("  Replaced by: {by}");
    } else {
        anyhow::bail!("Supersede is not supported for this artifact type: {id}");
    }

    Ok(vec![])
}
