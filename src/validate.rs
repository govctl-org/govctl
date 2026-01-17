//! Schema validation and state machine rules.
//!
//! Implements validation per [[RFC-0000]] and [[RFC-0001]]:
//! - [[ADR-0003]] signature verification for rendered projections
//! - [[ADR-0010]] placeholder description detection

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::find_clause_json;
use crate::model::{
    AdrStatus, ClauseStatus, ProjectIndex, RfcIndex, RfcPhase, RfcStatus, WorkItemStatus,
};
use crate::signature::{compute_rfc_signature, extract_signature};
use crate::write::read_clause;

// =============================================================================
// Field Validation System
// =============================================================================

/// Context for field validation
pub struct ValidationContext<'a> {
    pub config: &'a Config,
    /// The artifact being modified (e.g., "RFC-0001:C-NAME")
    pub artifact_id: &'a str,
}

/// Artifact kinds for validation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Adr and WorkItem will be used as validation expands
pub enum ArtifactKind {
    Rfc,
    Clause,
    Adr,
    WorkItem,
}

/// Field validation rules
#[derive(Debug, Clone)]
pub enum FieldValidation {
    /// No validation required
    None,
    /// Must be valid semver (e.g., "1.2.3")
    Semver,
    /// Must be a valid clause reference within same RFC, target must be active
    ClauseSupersededBy,
    /// Must be a valid artifact reference (RFC-xxx, ADR-xxx, etc.)
    ArtifactRef,
    /// Must be a valid enum value (validated by serde)
    EnumValue,
}

impl FieldValidation {
    /// Get the validation rule for a field
    pub fn for_field(kind: ArtifactKind, field: &str) -> Self {
        match (kind, field) {
            // Clause fields
            (ArtifactKind::Clause, "since") => Self::Semver,
            (ArtifactKind::Clause, "superseded_by") => Self::ClauseSupersededBy,
            (ArtifactKind::Clause, "status") => Self::EnumValue,
            (ArtifactKind::Clause, "kind") => Self::EnumValue,

            // RFC fields
            (ArtifactKind::Rfc, "version") => Self::Semver,
            (ArtifactKind::Rfc, "status") => Self::EnumValue,
            (ArtifactKind::Rfc, "phase") => Self::EnumValue,

            // ADR fields
            (ArtifactKind::Adr, "status") => Self::EnumValue,
            (ArtifactKind::Adr, "superseded_by") => Self::ArtifactRef,

            // Work item fields
            (ArtifactKind::WorkItem, "status") => Self::EnumValue,

            // Default: no validation
            _ => Self::None,
        }
    }

    /// Validate a value
    pub fn validate(&self, ctx: &ValidationContext, value: &str) -> anyhow::Result<()> {
        match self {
            Self::None => Ok(()),
            Self::EnumValue => Ok(()), // Validated by serde during parse
            Self::Semver => validate_semver(value),
            Self::ClauseSupersededBy => validate_clause_superseded_by(ctx, value),
            Self::ArtifactRef => validate_artifact_ref(ctx.config, value),
        }
    }
}

/// Validate a semver string
fn validate_semver(value: &str) -> anyhow::Result<()> {
    semver::Version::parse(value).map_err(|_| anyhow::anyhow!("Invalid semver: {value}"))?;
    Ok(())
}

/// Validate a clause superseded_by reference
fn validate_clause_superseded_by(ctx: &ValidationContext, target: &str) -> anyhow::Result<()> {
    // Empty string means "clear the field"
    if target.is_empty() {
        return Ok(());
    }

    // Extract RFC ID from source clause (e.g., "RFC-0001:C-NAME" -> "RFC-0001")
    let source_rfc = ctx
        .artifact_id
        .split(':')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid clause ID format: {}", ctx.artifact_id))?;

    // Build full target reference
    let full_target = if target.contains(':') {
        target.to_string()
    } else {
        format!("{source_rfc}:{target}")
    };

    // Check target is in same RFC
    let target_rfc = full_target
        .split(':')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid target clause ID: {target}"))?;

    if target_rfc != source_rfc {
        return Err(Diagnostic::new(
            DiagnosticCode::E0206ClauseSupersededByUnknown,
            format!(
                "superseded_by must reference a clause in the same RFC (got {target_rfc}, expected {source_rfc})"
            ),
            target,
        )
        .into());
    }

    // Check target clause exists
    let target_path = find_clause_json(ctx.config, &full_target).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Target clause not found: {full_target}"),
            &full_target,
        )
    })?;

    // Check target clause is active (not superseded or deprecated)
    let target_clause = read_clause(&target_path)?;
    match target_clause.status {
        ClauseStatus::Active => Ok(()),
        ClauseStatus::Superseded => Err(Diagnostic::new(
            DiagnosticCode::E0207ClauseSupersededByNotActive,
            format!("Cannot supersede by a superseded clause: {full_target}"),
            &full_target,
        )
        .into()),
        ClauseStatus::Deprecated => Err(Diagnostic::new(
            DiagnosticCode::E0207ClauseSupersededByNotActive,
            format!("Cannot supersede by a deprecated clause: {full_target}"),
            &full_target,
        )
        .into()),
    }
}

/// Validate an artifact reference exists
fn validate_artifact_ref(config: &Config, ref_id: &str) -> anyhow::Result<()> {
    use crate::load::find_rfc_json;
    use crate::parse::{load_adrs, load_work_items};

    if ref_id.starts_with("RFC-") {
        if find_rfc_json(config, ref_id).is_none() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else if ref_id.starts_with("ADR-") {
        let adrs = load_adrs(config)?;
        if !adrs.iter().any(|a| a.spec.govctl.id == ref_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else if ref_id.starts_with("WI-") {
        let items = load_work_items(config)?;
        if !items.iter().any(|w| w.spec.govctl.id == ref_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Unknown artifact type: {ref_id}"),
            ref_id,
        )
        .into());
    }
    Ok(())
}

// =============================================================================
// Convenience function for commands
// =============================================================================

/// Validate a field value before setting
pub fn validate_field(
    config: &Config,
    artifact_id: &str,
    kind: ArtifactKind,
    field: &str,
    value: &str,
) -> anyhow::Result<()> {
    let ctx = ValidationContext {
        config,
        artifact_id,
    };
    let validation = FieldValidation::for_field(kind, field);
    validation.validate(&ctx, value)
}

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
        validate_rfc(rfc, &mut result);
    }

    // Validate RFC signatures (per ADR-0003)
    validate_rfc_signatures(index, config, &mut result);

    // Validate cross-references
    validate_clause_references(index, &mut result);

    // Validate ADRs
    for adr in &index.adrs {
        if adr.meta().refs.is_empty() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                "ADR does not reference any artifacts (refs field is empty)",
                adr.path.display().to_string(),
            ));
        }

        // Validate content is not placeholder
        if adr.spec.content.context.contains("Describe the context") {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                "ADR context appears to be placeholder text",
                adr.path.display().to_string(),
            ));
        }
    }

    // Validate artifact references (refs fields)
    validate_artifact_refs(index, &mut result);

    // Validate work item descriptions
    validate_work_item_descriptions(index, &mut result);

    result
}

/// Validate RFC rendered markdown signatures (per ADR-0003)
fn validate_rfc_signatures(index: &ProjectIndex, config: &Config, result: &mut ValidationResult) {
    let output_dir = config.rfc_output();

    for rfc in &index.rfcs {
        let md_path = output_dir.join(format!("{}.md", rfc.rfc.rfc_id));

        // Skip if rendered file doesn't exist yet
        if !md_path.exists() {
            continue;
        }

        // Read rendered markdown
        let md_content = match std::fs::read_to_string(&md_path) {
            Ok(content) => content,
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::W0106RenderedReadError,
                    format!("Could not read rendered markdown: {}", e),
                    md_path.display().to_string(),
                ));
                continue;
            }
        };

        // Extract signature from rendered markdown
        let Some(existing_sig) = extract_signature(&md_content) else {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0602SignatureMissing,
                format!(
                    "Rendered markdown missing signature. Run 'govctl render' to regenerate: {}",
                    rfc.rfc.rfc_id
                ),
                md_path.display().to_string(),
            ));
            continue;
        };

        // Compute expected signature from source
        let expected_sig = match compute_rfc_signature(rfc) {
            Ok(sig) => sig,
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0601SignatureMismatch,
                    format!("Failed to compute signature for {}: {}", rfc.rfc.rfc_id, e),
                    md_path.display().to_string(),
                ));
                continue;
            }
        };

        // Compare
        if existing_sig != expected_sig {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0601SignatureMismatch,
                format!(
                    "Signature mismatch: rendered markdown was edited directly or source changed. Run 'govctl render' to regenerate: {}",
                    rfc.rfc.rfc_id
                ),
                md_path.display().to_string(),
            ));
        }
    }
}

/// Validate a single RFC
fn validate_rfc(rfc: &RfcIndex, result: &mut ValidationResult) {
    // Check RFC ID matches directory
    let dir_name = rfc
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    if let Some(name) = dir_name {
        if name != rfc.rfc.rfc_id {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0103RfcIdMismatch,
                format!(
                    "RFC ID '{}' doesn't match directory '{}'",
                    rfc.rfc.rfc_id, name
                ),
                rfc.path.display().to_string(),
            ));
        }
    }

    // Check changelog exists
    if rfc.rfc.changelog.is_empty() {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::W0101RfcNoChangelog,
            "RFC has no changelog entries",
            rfc.path.display().to_string(),
        ));
    }

    // Validate status/phase constraints
    validate_status_phase_constraints(rfc, result);

    // Validate clauses
    for clause in &rfc.clauses {
        // Check clause has 'since' field
        if clause.spec.since.is_none() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0102ClauseNoSince,
                format!("Clause '{}' has no 'since' version", clause.spec.clause_id),
                clause.path.display().to_string(),
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
                clause.path.display().to_string(),
            ));
        }
    }
}

/// Validate status/phase constraints per RFC-0000
fn validate_status_phase_constraints(rfc: &RfcIndex, result: &mut ValidationResult) {
    let status = rfc.rfc.status;
    let phase = rfc.rfc.phase;

    // draft + stable is forbidden
    if status == RfcStatus::Draft && phase == RfcPhase::Stable {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            "Cannot have status=draft with phase=stable",
            rfc.path.display().to_string(),
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
            rfc.path.display().to_string(),
        ));
    }
}

/// Validate clause cross-references (superseded_by)
fn validate_clause_references(index: &ProjectIndex, result: &mut ValidationResult) {
    // Collect all active clause IDs
    let active_clauses: std::collections::HashSet<String> = index
        .iter_clauses()
        .filter(|(_, c)| c.spec.status == ClauseStatus::Active)
        .map(|(rfc, c)| format!("{}:{}", rfc.rfc.rfc_id, c.spec.clause_id))
        .collect();

    // Check superseded_by references
    for (rfc, clause) in index.iter_clauses() {
        if let Some(ref superseded_by) = clause.spec.superseded_by {
            // If superseded, status should be Superseded
            if clause.spec.status != ClauseStatus::Superseded {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0206ClauseSupersededByUnknown,
                    format!(
                        "Clause has superseded_by but status is not 'superseded': {}",
                        clause.spec.clause_id
                    ),
                    clause.path.display().to_string(),
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
                    clause.path.display().to_string(),
                ));
            }
        }
    }
}

/// Validate refs fields in RFCs, ADRs and Work Items
fn validate_artifact_refs(index: &ProjectIndex, result: &mut ValidationResult) {
    // Build a set of all known artifact IDs (including clause references)
    let mut known_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Add RFC IDs and clause references
    for rfc in &index.rfcs {
        known_ids.insert(rfc.rfc.rfc_id.clone());
        // Add clause references in format RFC-ID:CLAUSE-ID
        for clause in &rfc.clauses {
            known_ids.insert(format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id));
        }
    }

    // Add ADR IDs
    for adr in &index.adrs {
        known_ids.insert(adr.meta().id.clone());
    }

    // Add Work Item IDs
    for work in &index.work_items {
        known_ids.insert(work.meta().id.clone());
    }

    // Validate RFC refs and supersedes
    for rfc in &index.rfcs {
        // Validate refs field
        for ref_id in &rfc.rfc.refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0105RfcRefNotFound,
                    format!(
                        "RFC '{}' references unknown artifact: {}",
                        rfc.rfc.rfc_id, ref_id
                    ),
                    rfc.path.display().to_string(),
                ));
            }
        }

        // Validate supersedes field
        if let Some(ref supersedes) = rfc.rfc.supersedes {
            if !known_ids.contains(supersedes) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0106RfcSupersedesNotFound,
                    format!(
                        "RFC '{}' supersedes unknown RFC: {}",
                        rfc.rfc.rfc_id, supersedes
                    ),
                    rfc.path.display().to_string(),
                ));
            }
        }
    }

    // Validate ADR refs
    for adr in &index.adrs {
        for ref_id in &adr.meta().refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0304AdrRefNotFound,
                    format!(
                        "ADR '{}' references unknown artifact: {}",
                        adr.meta().id,
                        ref_id
                    ),
                    adr.path.display().to_string(),
                ));
            }
        }
    }

    // Validate Work Item refs
    for work in &index.work_items {
        for ref_id in &work.meta().refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0404WorkRefNotFound,
                    format!(
                        "Work item '{}' references unknown artifact: {}",
                        work.meta().id,
                        ref_id
                    ),
                    work.path.display().to_string(),
                ));
            }
        }
    }
}

/// Check if a work item description is a placeholder or empty
fn is_placeholder_description(desc: &str) -> bool {
    let trimmed = desc.trim();

    // Empty or whitespace-only
    if trimmed.is_empty() {
        return true;
    }

    // Exact template match
    if trimmed.contains("Describe the work to be done")
        && trimmed.contains("What is the goal?")
        && trimmed.contains("What are the acceptance criteria?")
    {
        return true;
    }

    // Common placeholder patterns (case-insensitive)
    let lower = trimmed.to_lowercase();
    let placeholder_patterns = ["todo", "tbd", "fill in later", "placeholder", "fixme"];

    // Only flag if the entire description is just a placeholder word
    placeholder_patterns
        .iter()
        .any(|p| lower == *p || lower == format!("[{}]", p) || lower == format!("<{}>", p))
}

/// Validate work item descriptions for placeholder content (per ADR-0010)
fn validate_work_item_descriptions(index: &ProjectIndex, result: &mut ValidationResult) {
    for work in &index.work_items {
        let desc = &work.spec.content.description;
        if is_placeholder_description(desc) {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0108WorkPlaceholderDescription,
                format!(
                    "Work item '{}' has placeholder or empty description",
                    work.meta().id
                ),
                work.path.display().to_string(),
            ));
        }
    }
}

/// Check if RFC status transition is valid
pub fn is_valid_status_transition(from: RfcStatus, to: RfcStatus) -> bool {
    matches!(
        (from, to),
        (RfcStatus::Draft, RfcStatus::Normative) | (RfcStatus::Normative, RfcStatus::Deprecated)
    )
}

/// Check if RFC phase transition is valid
pub fn is_valid_phase_transition(from: RfcPhase, to: RfcPhase) -> bool {
    matches!(
        (from, to),
        (RfcPhase::Spec, RfcPhase::Impl)
            | (RfcPhase::Impl, RfcPhase::Test)
            | (RfcPhase::Test, RfcPhase::Stable)
    )
}

/// Check if ADR status transition is valid
/// ADR lifecycle: proposed → accepted → superseded
///                        → rejected
pub fn is_valid_adr_transition(from: AdrStatus, to: AdrStatus) -> bool {
    matches!(
        (from, to),
        (AdrStatus::Proposed, AdrStatus::Accepted)
            | (AdrStatus::Proposed, AdrStatus::Rejected)
            | (AdrStatus::Accepted, AdrStatus::Superseded)
    )
}

/// Check if Work Item status transition is valid
pub fn is_valid_work_transition(from: WorkItemStatus, to: WorkItemStatus) -> bool {
    matches!(
        (from, to),
        (WorkItemStatus::Queue, WorkItemStatus::Active)
            | (WorkItemStatus::Active, WorkItemStatus::Done)
            | (WorkItemStatus::Queue, WorkItemStatus::Cancelled)
            | (WorkItemStatus::Active, WorkItemStatus::Cancelled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // RFC Status Transition Tests
    // =========================================================================

    #[test]
    fn test_rfc_status_draft_to_normative() {
        assert!(is_valid_status_transition(
            RfcStatus::Draft,
            RfcStatus::Normative
        ));
    }

    #[test]
    fn test_rfc_status_normative_to_deprecated() {
        assert!(is_valid_status_transition(
            RfcStatus::Normative,
            RfcStatus::Deprecated
        ));
    }

    #[test]
    fn test_rfc_status_invalid_draft_to_deprecated() {
        assert!(!is_valid_status_transition(
            RfcStatus::Draft,
            RfcStatus::Deprecated
        ));
    }

    #[test]
    fn test_rfc_status_invalid_normative_to_draft() {
        assert!(!is_valid_status_transition(
            RfcStatus::Normative,
            RfcStatus::Draft
        ));
    }

    #[test]
    fn test_rfc_status_invalid_deprecated_to_normative() {
        assert!(!is_valid_status_transition(
            RfcStatus::Deprecated,
            RfcStatus::Normative
        ));
    }

    #[test]
    fn test_rfc_status_same_state() {
        assert!(!is_valid_status_transition(
            RfcStatus::Draft,
            RfcStatus::Draft
        ));
        assert!(!is_valid_status_transition(
            RfcStatus::Normative,
            RfcStatus::Normative
        ));
    }

    // =========================================================================
    // RFC Phase Transition Tests
    // =========================================================================

    #[test]
    fn test_rfc_phase_spec_to_impl() {
        assert!(is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Impl));
    }

    #[test]
    fn test_rfc_phase_impl_to_test() {
        assert!(is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Test));
    }

    #[test]
    fn test_rfc_phase_test_to_stable() {
        assert!(is_valid_phase_transition(RfcPhase::Test, RfcPhase::Stable));
    }

    #[test]
    fn test_rfc_phase_invalid_skip() {
        // Cannot skip phases
        assert!(!is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Test));
        assert!(!is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Stable));
        assert!(!is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Stable));
    }

    #[test]
    fn test_rfc_phase_invalid_backward() {
        assert!(!is_valid_phase_transition(RfcPhase::Stable, RfcPhase::Test));
        assert!(!is_valid_phase_transition(RfcPhase::Test, RfcPhase::Impl));
        assert!(!is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Spec));
    }

    // =========================================================================
    // ADR Status Transition Tests
    // =========================================================================

    #[test]
    fn test_adr_status_proposed_to_accepted() {
        assert!(is_valid_adr_transition(
            AdrStatus::Proposed,
            AdrStatus::Accepted
        ));
    }

    #[test]
    fn test_adr_status_accepted_to_superseded() {
        assert!(is_valid_adr_transition(
            AdrStatus::Accepted,
            AdrStatus::Superseded
        ));
    }

    #[test]
    fn test_adr_status_proposed_to_rejected() {
        assert!(is_valid_adr_transition(
            AdrStatus::Proposed,
            AdrStatus::Rejected
        ));
    }

    #[test]
    fn test_adr_status_invalid_proposed_to_superseded() {
        assert!(!is_valid_adr_transition(
            AdrStatus::Proposed,
            AdrStatus::Superseded
        ));
    }

    #[test]
    fn test_adr_status_invalid_rejected_transitions() {
        // Rejected is terminal
        assert!(!is_valid_adr_transition(
            AdrStatus::Rejected,
            AdrStatus::Accepted
        ));
        assert!(!is_valid_adr_transition(
            AdrStatus::Rejected,
            AdrStatus::Proposed
        ));
    }

    #[test]
    fn test_adr_status_invalid_backward() {
        assert!(!is_valid_adr_transition(
            AdrStatus::Accepted,
            AdrStatus::Proposed
        ));
        assert!(!is_valid_adr_transition(
            AdrStatus::Superseded,
            AdrStatus::Accepted
        ));
    }

    // =========================================================================
    // Work Item Status Transition Tests
    // =========================================================================

    #[test]
    fn test_work_status_queue_to_active() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Queue,
            WorkItemStatus::Active
        ));
    }

    #[test]
    fn test_work_status_active_to_done() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Active,
            WorkItemStatus::Done
        ));
    }

    #[test]
    fn test_work_status_queue_to_cancelled() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Queue,
            WorkItemStatus::Cancelled
        ));
    }

    #[test]
    fn test_work_status_active_to_cancelled() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Active,
            WorkItemStatus::Cancelled
        ));
    }

    #[test]
    fn test_work_status_invalid_queue_to_done() {
        // Cannot skip active
        assert!(!is_valid_work_transition(
            WorkItemStatus::Queue,
            WorkItemStatus::Done
        ));
    }

    #[test]
    fn test_work_status_invalid_done_transitions() {
        // Done is terminal (except requeue which isn't implemented)
        assert!(!is_valid_work_transition(
            WorkItemStatus::Done,
            WorkItemStatus::Active
        ));
        assert!(!is_valid_work_transition(
            WorkItemStatus::Done,
            WorkItemStatus::Queue
        ));
    }

    #[test]
    fn test_work_status_invalid_cancelled_transitions() {
        // Cancelled is terminal
        assert!(!is_valid_work_transition(
            WorkItemStatus::Cancelled,
            WorkItemStatus::Active
        ));
        assert!(!is_valid_work_transition(
            WorkItemStatus::Cancelled,
            WorkItemStatus::Queue
        ));
    }

    // =========================================================================
    // FieldValidation::for_field Tests
    // =========================================================================

    #[test]
    fn test_field_validation_clause_since() {
        matches!(
            FieldValidation::for_field(ArtifactKind::Clause, "since"),
            FieldValidation::Semver
        );
    }

    #[test]
    fn test_field_validation_clause_superseded_by() {
        matches!(
            FieldValidation::for_field(ArtifactKind::Clause, "superseded_by"),
            FieldValidation::ClauseSupersededBy
        );
    }

    #[test]
    fn test_field_validation_rfc_version() {
        matches!(
            FieldValidation::for_field(ArtifactKind::Rfc, "version"),
            FieldValidation::Semver
        );
    }

    #[test]
    fn test_field_validation_unknown_field() {
        matches!(
            FieldValidation::for_field(ArtifactKind::Rfc, "unknown"),
            FieldValidation::None
        );
    }
}
