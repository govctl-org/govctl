//! Lifecycle command implementations.

use crate::FinalizeStatus;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::{find_clause_json, find_rfc_json};
use crate::model::{AdrStatus, ClauseStatus, Release, RfcPhase, RfcStatus, WorkItemStatus};
use crate::parse::{
    load_adrs, load_releases, load_work_items, validate_version, write_adr, write_releases,
};
use crate::ui;
use crate::validate::{
    is_valid_adr_transition, is_valid_phase_transition, is_valid_status_transition,
};
use crate::write::{
    BumpLevel, WriteOp, add_changelog_change, bump_rfc_version, read_clause, read_rfc, today,
    write_clause, write_rfc,
};
use std::collections::HashSet;

/// Bump RFC version
pub fn bump(
    config: &Config,
    rfc_id: &str,
    level: Option<BumpLevel>,
    summary: Option<&str>,
    changes: &[String],
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path = find_rfc_json(config, rfc_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {rfc_id}"),
            rfc_id,
        )
    })?;

    let mut rfc = read_rfc(&rfc_path)?;

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
            
            // Recompute and store signature after version bump per [[ADR-0016]]
            // Load full RFC with clauses to compute accurate signature
            if let Ok(rfc_index) = crate::load::load_rfc(&rfc_path) {
                if let Ok(sig) = crate::signature::compute_rfc_signature(&rfc_index) {
                    rfc.signature = Some(sig);
                }
            }
        }
        (Some(_), None, _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0108RfcBumpRequiresSummary,
                "--summary is required when bumping version",
                rfc_id,
            )
            .into());
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
            )
            .into());
        }
        (None, None, true) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0801MissingRequiredArg,
                "Provide bump level with --summary, or --change",
                rfc_id,
            )
            .into());
        }
    }

    write_rfc(&rfc_path, &rfc, op)?;
    Ok(vec![])
}

/// Finalize RFC status
pub fn finalize(
    config: &Config,
    rfc_id: &str,
    status: FinalizeStatus,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path = find_rfc_json(config, rfc_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {rfc_id}"),
            rfc_id,
        )
    })?;

    let mut rfc = read_rfc(&rfc_path)?;

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
        )
        .into());
    }

    rfc.status = target_status;
    rfc.updated = Some(crate::write::today());
    write_rfc(&rfc_path, &rfc, op)?;

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
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_path = find_rfc_json(config, rfc_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {rfc_id}"),
            rfc_id,
        )
    })?;

    let mut rfc = read_rfc(&rfc_path)?;

    // Check status constraint: cannot advance to impl+ without normative status
    if rfc.status == RfcStatus::Draft && phase != RfcPhase::Spec {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot advance to {} while status is draft. Finalize to normative first.",
                phase.as_ref()
            ),
            rfc_id,
        )
        .into());
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
        )
        .into());
    }

    rfc.phase = phase;
    rfc.updated = Some(crate::write::today());
    write_rfc(&rfc_path, &rfc, op)?;

    if !op.is_preview() {
        ui::phase_advanced(rfc_id, phase.as_ref());
    }
    Ok(vec![])
}

/// Accept an ADR
pub fn accept_adr(config: &Config, adr_id: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let mut entry = load_adrs(config)?
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

    entry.spec.govctl.status = AdrStatus::Accepted;
    write_adr(&entry.path, &entry.spec, op)?;

    if !op.is_preview() {
        ui::accepted("ADR", adr_id);
    }
    Ok(vec![])
}

/// Reject an ADR
pub fn reject_adr(config: &Config, adr_id: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let mut entry = load_adrs(config)?
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

    entry.spec.govctl.status = AdrStatus::Rejected;
    write_adr(&entry.path, &entry.spec, op)?;

    if !op.is_preview() {
        ui::rejected("ADR", adr_id);
    }
    Ok(vec![])
}

/// Deprecate an artifact
pub fn deprecate(config: &Config, id: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    if id.contains(':') {
        // It's a clause
        let clause_path = find_clause_json(config, id).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {id}"),
                id,
            )
        })?;

        let mut clause = read_clause(&clause_path)?;

        if clause.status == ClauseStatus::Deprecated {
            return Err(Diagnostic::new(
                DiagnosticCode::E0208ClauseAlreadyDeprecated,
                "Clause is already deprecated",
                id,
            )
            .into());
        }
        if clause.status == ClauseStatus::Superseded {
            return Err(Diagnostic::new(
                DiagnosticCode::E0209ClauseAlreadySuperseded,
                "Clause is superseded, cannot deprecate",
                id,
            )
            .into());
        }

        clause.status = ClauseStatus::Deprecated;
        write_clause(&clause_path, &clause, op)?;

        if !op.is_preview() {
            ui::deprecated("clause", id);
        }
    } else if id.starts_with("RFC-") {
        // Use finalize for RFC deprecation
        return finalize(config, id, FinalizeStatus::Deprecated, op);
    } else if id.starts_with("ADR-") {
        // ADRs cannot be deprecated; they can only be superseded
        return Err(Diagnostic::new(
            DiagnosticCode::E0305AdrCannotDeprecate,
            format!(
                "ADRs cannot be deprecated. Use `govctl supersede {id} --by ADR-XXXX` instead."
            ),
            id,
        )
        .into());
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Unknown artifact type: {id}"),
            id,
        )
        .into());
    }

    Ok(vec![])
}

/// Supersede an artifact
pub fn supersede(
    config: &Config,
    id: &str,
    by: &str,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    if id.contains(':') {
        // It's a clause
        // Validate replacement exists
        let _ = find_clause_json(config, by).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Replacement clause not found: {by}"),
                by,
            )
        })?;

        let clause_path = find_clause_json(config, id).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {id}"),
                id,
            )
        })?;

        let mut clause = read_clause(&clause_path)?;

        if clause.status == ClauseStatus::Superseded {
            return Err(Diagnostic::new(
                DiagnosticCode::E0209ClauseAlreadySuperseded,
                "Clause is already superseded",
                id,
            )
            .into());
        }

        clause.status = ClauseStatus::Superseded;
        clause.superseded_by = Some(by.to_string());
        write_clause(&clause_path, &clause, op)?;

        if !op.is_preview() {
            ui::superseded("clause", id, by);
        }
    } else if id.starts_with("ADR-") {
        // Load all ADRs once and find both source and replacement
        let adrs = load_adrs(config)?;

        // Validate replacement exists
        let _ = adrs
            .iter()
            .find(|a| a.spec.govctl.id == by)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0302AdrNotFound,
                    format!("Replacement ADR not found: {by}"),
                    by,
                )
            })?;

        // Find the ADR to supersede
        let mut entry = adrs
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0302AdrNotFound,
                    format!("ADR not found: {id}"),
                    id,
                )
            })?;

        if !is_valid_adr_transition(entry.spec.govctl.status, AdrStatus::Superseded) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0303AdrInvalidTransition,
                format!(
                    "Invalid ADR transition: {} -> superseded",
                    entry.spec.govctl.status.as_ref()
                ),
                id,
            )
            .into());
        }

        entry.spec.govctl.status = AdrStatus::Superseded;
        entry.spec.govctl.superseded_by = Some(by.to_string());
        write_adr(&entry.path, &entry.spec, op)?;

        if !op.is_preview() {
            ui::superseded("ADR", id, by);
        }
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Supersede is not supported for this artifact type: {id}"),
            id,
        )
        .into());
    }

    Ok(vec![])
}

/// Cut a release - collect unreleased work items into a version
/// Per [[ADR-0014]], stores release info in gov/releases.toml
pub fn cut_release(
    config: &Config,
    version: &str,
    date: Option<&str>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let releases_path = config.releases_path();
    let releases_path_str = releases_path.display().to_string();

    // Validate version is valid semver
    validate_version(version).map_err(|_| {
        let diag = Diagnostic::new(
            DiagnosticCode::E0701ReleaseInvalidSemver,
            format!("Invalid semver version: {version}"),
            &releases_path_str,
        );
        anyhow::anyhow!("{}", diag)
    })?;

    // Load existing releases
    let mut releases_file = load_releases(config).map_err(|d| anyhow::anyhow!("{}", d))?;

    // Check for duplicate version
    if releases_file.releases.iter().any(|r| r.version == version) {
        let diag = Diagnostic::new(
            DiagnosticCode::E0702ReleaseDuplicate,
            format!("Release {version} already exists"),
            &releases_path_str,
        );
        anyhow::bail!("{}", diag);
    }

    // Get all work item IDs already in releases
    let released_ids: HashSet<_> = releases_file
        .releases
        .iter()
        .flat_map(|r| r.refs.iter().cloned())
        .collect();

    // Load all done work items
    let work_items = load_work_items(config).map_err(|d| anyhow::anyhow!("{}", d))?;
    let unreleased: Vec<_> = work_items
        .iter()
        .filter(|w| w.spec.govctl.status == WorkItemStatus::Done)
        .filter(|w| !released_ids.contains(&w.spec.govctl.id))
        .collect();

    if unreleased.is_empty() {
        let diag = Diagnostic::new(
            DiagnosticCode::E0703ReleaseNoUnreleasedItems,
            "No unreleased work items to include in release",
            &releases_path_str,
        );
        anyhow::bail!("{}", diag);
    }

    // Create new release
    let release_date = date.map(|d| d.to_string()).unwrap_or_else(today);
    let mut refs: Vec<_> = unreleased
        .iter()
        .map(|w| w.spec.govctl.id.clone())
        .collect();
    refs.sort(); // Ensure deterministic ordering across platforms

    let release = Release {
        version: version.to_string(),
        date: release_date.clone(),
        refs: refs.clone(),
    };

    // Insert at the beginning (newest first)
    releases_file.releases.insert(0, release);

    // Write releases file
    write_releases(config, &releases_file, op).map_err(|d| anyhow::anyhow!("{}", d))?;

    if !op.is_preview() {
        ui::release_created(version, &release_date, refs.len());
    }

    Ok(vec![])
}
