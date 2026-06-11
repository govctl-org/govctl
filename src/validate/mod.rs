//! Schema validation and state machine rules.
//!
//! Implements validation per [[RFC-0000]] and [[RFC-0001]]:
//! - [[ADR-0003]] signature verification for rendered projections
//! - [[ADR-0010]] placeholder description detection
//! - [[RFC-0000:C-REFERENCE-HIERARCHY]] structured refs, [[...]] link targets, and bare known IDs

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;

mod artifact_refs;
mod bracket_refs;
mod fields;
mod lifecycle;
mod reference_hierarchy;
mod releases;
mod rfc;
mod signatures;
mod tags;
mod work_dependencies;
mod work_items;

use artifact_refs::validate_artifact_refs;
use bracket_refs::validate_bracket_reference_hierarchy;
use rfc::{validate_clause_references, validate_rfc};
use signatures::validate_rfc_signatures;
use tags::validate_artifact_tags;
use work_items::{validate_work_item_descriptions, validate_work_item_legacy_inline_history};

pub use artifact_refs::validate_artifact_ref_edit;
pub use fields::{ArtifactKind, validate_field};
pub use lifecycle::{
    is_valid_adr_transition, is_valid_phase_transition, is_valid_status_transition,
    is_valid_work_transition,
};
pub use releases::validate_releases;
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

    // Inline reference syntax in governed prose — [[RFC-0000:C-REFERENCE-HIERARCHY]]
    validate_bracket_reference_hierarchy(index, config, &mut result);

    // Validate work item descriptions
    validate_work_item_descriptions(index, config, &mut result);

    // Surface legacy inline execution history without blocking validation.
    validate_work_item_legacy_inline_history(index, config, &mut result);

    // Validate tags against allowed set — [[RFC-0002:C-RESOURCES]]
    validate_artifact_tags(index, config, &mut result);

    result
}
