//! Schema validation and state machine rules.
//!
//! Implements validation per [[RFC-0000]] and [[RFC-0001]]:
//! - [[ADR-0003]] signature verification for rendered projections
//! - [[ADR-0010]] placeholder description detection
//! - [[RFC-0000:C-REFERENCE-HIERARCHY]] structured refs and [[...]] link targets

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseStatus, ProjectIndex, ReleasesFile, RfcIndex, RfcPhase, RfcStatus};
use std::collections::HashSet;

mod artifact_refs;
mod bracket_refs;
mod fields;
mod lifecycle;
mod signatures;
mod work_dependencies;
mod work_items;

use artifact_refs::validate_artifact_refs;
use bracket_refs::validate_bracket_reference_hierarchy;
use signatures::validate_rfc_signatures;
use work_items::{validate_work_item_descriptions, validate_work_item_legacy_inline_history};

pub use fields::{ArtifactKind, validate_field};
pub use lifecycle::{
    is_valid_adr_transition, is_valid_phase_transition, is_valid_status_transition,
    is_valid_work_transition,
};
pub use work_dependencies::{is_work_item_id, validate_work_dependencies};

/// Validation result with diagnostics
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    pub rfc_count: usize,
    pub clause_count: usize,
    pub adr_count: usize,
    pub work_count: usize,
}

impl ValidationResult {
    #[allow(dead_code)]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == crate::diagnostic::DiagnosticLevel::Error)
    }
}

/// Validate the entire project
pub fn validate_project(index: &ProjectIndex, config: &Config) -> ValidationResult {
    let mut result = ValidationResult {
        rfc_count: index.rfcs.len(),
        clause_count: index.iter_clauses().count(),
        adr_count: index.adrs.len(),
        work_count: index.work_items.len(),
        ..Default::default()
    };

    // Validate RFCs
    for rfc in &index.rfcs {
        validate_rfc(rfc, config, &mut result);
    }

    // Validate RFC signatures (per ADR-0003)
    validate_rfc_signatures(index, config, &mut result);

    // Validate cross-references
    validate_clause_references(index, config, &mut result);

    // Validate ADRs
    for adr in &index.adrs {
        let adr_path_display = config.display_path(&adr.path).display().to_string();
        if adr.meta().refs.is_empty() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                format!(
                    "ADR has no artifact references (hint: `govctl adr add {} refs RFC-XXXX`)",
                    adr.meta().id
                ),
                adr_path_display.clone(),
            ));
        }

        // Validate content is not placeholder
        if adr.spec.content.context.contains("Describe the context") {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                format!(
                    "ADR has placeholder context (hint: `govctl adr set {} context \"...\"`)",
                    adr.meta().id
                ),
                adr_path_display,
            ));
        }
    }

    // Validate artifact references (refs fields)
    validate_artifact_refs(index, config, &mut result);

    // Validate work item dependency declarations per [[RFC-0006:C-DEPENDENCY-SEMANTICS]]
    result
        .diagnostics
        .extend(validate_work_dependencies(index, config));

    // [[...]] in RFC/ADR governed text — [[RFC-0000:C-REFERENCE-HIERARCHY]]
    validate_bracket_reference_hierarchy(index, config, &mut result);

    // Validate work item descriptions
    validate_work_item_descriptions(index, config, &mut result);

    // Surface legacy inline execution history without blocking validation.
    validate_work_item_legacy_inline_history(index, config, &mut result);

    // Validate tags against allowed set — [[RFC-0002:C-RESOURCES]]
    validate_artifact_tags(index, config, &mut result);

    result
}

/// Validate a single RFC
fn validate_rfc(rfc: &RfcIndex, config: &Config, result: &mut ValidationResult) {
    let rfc_path_display = config.display_path(&rfc.path).display().to_string();

    // Check RFC ID matches directory
    let dir_name = rfc
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    if let Some(name) = dir_name
        && name != rfc.rfc.rfc_id
    {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0103RfcIdMismatch,
            format!(
                "RFC ID '{}' doesn't match directory '{}'",
                rfc.rfc.rfc_id, name
            ),
            rfc_path_display.clone(),
        ));
    }

    // Check changelog exists
    if rfc.rfc.changelog.is_empty() {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::W0101RfcNoChangelog,
            "RFC has no changelog entries (hint: run `govctl rfc bump`)",
            rfc_path_display.clone(),
        ));
    }

    // Validate status/phase constraints
    validate_status_phase_constraints(rfc, config, result);

    // Validate clauses
    for clause in &rfc.clauses {
        let clause_path_display = config.display_path(&clause.path).display().to_string();
        // Check clause has 'since' field
        if clause.spec.since.is_none() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0102ClauseNoSince,
                format!(
                    "Clause '{}' has no 'since' version (hint: it will be set automatically by `govctl rfc bump` or `govctl rfc finalize`)",
                    clause.spec.clause_id
                ),
                clause_path_display.clone(),
            ));
        }

        // Check clause ID matches filename
        let file_name = clause
            .path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name != clause.spec.clause_id {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0203ClauseIdMismatch,
                format!(
                    "Clause ID '{}' doesn't match filename '{}'",
                    clause.spec.clause_id, file_name
                ),
                clause_path_display,
            ));
        }
    }
}

/// Validate status/phase constraints per RFC-0000
fn validate_status_phase_constraints(
    rfc: &RfcIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let status = rfc.rfc.status;
    let phase = rfc.rfc.phase;
    let path_display = config.display_path(&rfc.path).display().to_string();

    // draft + stable is forbidden
    if status == RfcStatus::Draft && phase == RfcPhase::Stable {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            "Cannot have status=draft with phase=stable",
            path_display.clone(),
        ));
    }

    // deprecated + impl/test is forbidden
    if status == RfcStatus::Deprecated && (phase == RfcPhase::Impl || phase == RfcPhase::Test) {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot have status=deprecated with phase={}",
                phase.as_ref()
            ),
            path_display,
        ));
    }
}

/// Validate clause cross-references (superseded_by)
fn validate_clause_references(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    // Collect all active clause IDs
    let active_clauses: HashSet<String> = index
        .iter_clauses()
        .filter(|(_, c)| c.spec.status == ClauseStatus::Active)
        .map(|(rfc, c)| format!("{}:{}", rfc.rfc.rfc_id, c.spec.clause_id))
        .collect();

    // Check superseded_by references
    for (rfc, clause) in index.iter_clauses() {
        if let Some(ref superseded_by) = clause.spec.superseded_by {
            let clause_path_display = config.display_path(&clause.path).display().to_string();
            // If superseded, status should be Superseded
            if clause.spec.status != ClauseStatus::Superseded {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0206ClauseSupersededByUnknown,
                    format!(
                        "Clause has superseded_by but status is not 'superseded': {}",
                        clause.spec.clause_id
                    ),
                    clause_path_display.clone(),
                ));
            }

            // Build full reference
            let full_ref = if superseded_by.contains(':') {
                superseded_by.clone()
            } else {
                format!("{}:{}", rfc.rfc.rfc_id, superseded_by)
            };

            // Check reference exists and is active
            if !active_clauses.contains(&full_ref) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0207ClauseSupersededByNotActive,
                    format!(
                        "Clause '{}' superseded by '{}' which is not active",
                        clause.spec.clause_id, superseded_by
                    ),
                    clause_path_display,
                ));
            }
        }
    }
}

/// Validate release metadata and work item references.
pub fn validate_releases(
    releases: &ReleasesFile,
    index: &ProjectIndex,
    config: &Config,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_versions = HashSet::new();
    let known_work_ids: HashSet<&str> = index
        .work_items
        .iter()
        .map(|work| work.meta().id.as_str())
        .collect();
    let releases_display = config
        .display_path(&config.releases_path())
        .display()
        .to_string();

    for release in &releases.releases {
        if !seen_versions.insert(release.version.as_str()) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0702ReleaseDuplicate,
                format!("Duplicate release version: {}", release.version),
                releases_display.clone(),
            ));
        }

        for work_id in &release.refs {
            if !known_work_ids.contains(work_id.as_str()) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0705ReleaseRefNotFound,
                    format!(
                        "Release '{}' references unknown work item: {}",
                        release.version, work_id
                    ),
                    releases_display.clone(),
                ));
            }
        }
    }

    diagnostics
}

/// Validate that all artifact tags are in the allowed set and well-formed.
///
/// Per [[RFC-0002:C-RESOURCES]] controlled-vocabulary tags: every tag used by an
/// artifact must be listed in config.toml [tags] allowed, and each tag must match
/// the format `^[a-z][a-z0-9-]*$`.
fn validate_artifact_tags(index: &ProjectIndex, config: &Config, result: &mut ValidationResult) {
    let allowed = &config.tags.allowed;

    let mut check_tags = |tags: &[String], artifact_id: &str, path_display: &str| {
        for tag in tags {
            // Validate format
            let tag_re = match crate::cmd::tag::tag_re() {
                Ok(r) => r,
                Err(e) => {
                    result.diagnostics.push(Diagnostic::new(
                        DiagnosticCode::E0806InvalidPattern,
                        format!("Failed to compile tag regex: {e}"),
                        path_display,
                    ));
                    continue;
                }
            };
            if !tag_re.is_match(tag) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E1101TagInvalidFormat,
                    format!(
                        "Artifact '{artifact_id}' has invalid tag format '{tag}': must match ^[a-z][a-z0-9-]*$"
                    ),
                    path_display,
                ));
                continue;
            }
            // Validate against allowed set (deny-all when empty)
            if !allowed.contains(tag) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E1105TagUnknown,
                    format!(
                        "Artifact '{artifact_id}' uses unknown tag '{tag}' (not in config.toml [tags] allowed)"
                    ),
                    path_display,
                ));
            }
        }
    };

    for rfc in &index.rfcs {
        let path = config.display_path(&rfc.path).display().to_string();
        check_tags(&rfc.rfc.tags, &rfc.rfc.rfc_id, &path);
    }

    for (rfc, clause) in index.iter_clauses() {
        let clause_id = format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id);
        let path = config.display_path(&clause.path).display().to_string();
        check_tags(&clause.spec.tags, &clause_id, &path);
    }

    for adr in &index.adrs {
        let path = config.display_path(&adr.path).display().to_string();
        check_tags(&adr.spec.govctl.tags, &adr.meta().id, &path);
    }

    for work in &index.work_items {
        let path = config.display_path(&work.path).display().to_string();
        check_tags(&work.spec.govctl.tags, &work.meta().id, &path);
    }

    if let Ok(guard_result) = crate::parse::load_guards_with_warnings(config) {
        for guard in &guard_result.items {
            let path = config.display_path(&guard.path).display().to_string();
            check_tags(&guard.spec.govctl.tags, &guard.meta().id, &path);
        }
    }
}
