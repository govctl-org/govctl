//! Schema validation and state machine rules.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{
    AdrStatus, ClauseStatus, ProjectIndex, RfcIndex, RfcPhase, RfcStatus, WorkItemStatus,
};

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
        if adr.meta.kind != "adr" {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0301AdrSchemaInvalid,
                format!("ADR kind must be 'adr', got '{}'", adr.meta.kind),
                adr.path.display().to_string(),
            ));
        }

        if adr.meta.refs.is_empty() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                "ADR has no references",
                adr.path.display().to_string(),
            ));
        }
    }

    // Validate Work Items
    for item in &index.work_items {
        if item.meta.kind != "work" {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!("Work item kind must be 'work', got '{}'", item.meta.kind),
                item.path.display().to_string(),
            ));
        }
    }

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
            format!("Cannot have status=deprecated with phase={}", phase.as_ref()),
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
        (WorkItemStatus::Queue, WorkItemStatus::Active) | (WorkItemStatus::Active, WorkItemStatus::Done)
    )
}
