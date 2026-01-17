//! Schema validation and state machine rules.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::find_clause_json;
use crate::model::{
    AdrStatus, ClauseStatus, ProjectIndex, RfcIndex, RfcPhase, RfcStatus, WorkItemStatus,
};
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
        anyhow::bail!(
            "superseded_by must reference a clause in the same RFC (got {target_rfc}, expected {source_rfc})"
        );
    }

    // Check target clause exists
    let target_path = find_clause_json(ctx.config, &full_target)
        .ok_or_else(|| anyhow::anyhow!("Target clause not found: {full_target}"))?;

    // Check target clause is active (not superseded or deprecated)
    let target_clause = read_clause(&target_path)?;
    match target_clause.status {
        ClauseStatus::Active => Ok(()),
        ClauseStatus::Superseded => {
            anyhow::bail!("Cannot supersede by a superseded clause: {full_target}")
        }
        ClauseStatus::Deprecated => {
            anyhow::bail!("Cannot supersede by a deprecated clause: {full_target}")
        }
    }
}

/// Validate an artifact reference exists
fn validate_artifact_ref(config: &Config, ref_id: &str) -> anyhow::Result<()> {
    use crate::load::find_rfc_json;
    use crate::parse::{load_adrs, load_work_items};

    if ref_id.starts_with("RFC-") {
        if find_rfc_json(config, ref_id).is_none() {
            anyhow::bail!("RFC not found: {ref_id}");
        }
    } else if ref_id.starts_with("ADR-") {
        let adrs = load_adrs(config)?;
        if !adrs.iter().any(|a| a.spec.govctl.id == ref_id) {
            anyhow::bail!("ADR not found: {ref_id}");
        }
    } else if ref_id.starts_with("WI-") {
        let items = load_work_items(config)?;
        if !items.iter().any(|w| w.spec.govctl.id == ref_id) {
            anyhow::bail!("Work item not found: {ref_id}");
        }
    } else {
        anyhow::bail!("Unknown artifact type: {ref_id}");
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
pub fn validate_project(index: &ProjectIndex, _config: &Config) -> ValidationResult {
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

    // Validate cross-references
    validate_clause_references(index, &mut result);

    // Validate ADRs
    for adr in &index.adrs {
        if adr.meta().refs.is_empty() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                "ADR has no references",
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

    // Validate Work Items (no specific checks currently)

    result
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
pub fn is_valid_adr_transition(from: AdrStatus, to: AdrStatus) -> bool {
    matches!(
        (from, to),
        (AdrStatus::Proposed, AdrStatus::Accepted)
            | (AdrStatus::Proposed, AdrStatus::Deprecated)
            | (AdrStatus::Accepted, AdrStatus::Superseded)
            | (AdrStatus::Accepted, AdrStatus::Deprecated)
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
