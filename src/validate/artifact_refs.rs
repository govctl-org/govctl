use super::ValidationResult;
use super::reference_hierarchy::{ReferenceSurface, check_ref_hierarchy};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;
use std::collections::HashSet;

/// Validate refs fields in RFCs, ADRs and Work Items
pub(super) fn validate_artifact_refs(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    // Build a set of all known artifact IDs (including clause references)
    let mut known_ids: HashSet<String> = HashSet::new();

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
        let rfc_path_display = config.display_path(&rfc.path).display().to_string();
        // Validate refs field
        for ref_id in &rfc.rfc.refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0105RfcRefNotFound,
                    format!(
                        "RFC '{}' references unknown artifact: {}",
                        rfc.rfc.rfc_id, ref_id
                    ),
                    rfc_path_display.clone(),
                ));
            } else if let Err(d) = check_ref_hierarchy(
                &rfc.rfc.rfc_id,
                ref_id,
                &rfc_path_display,
                ReferenceSurface::StructuredRef,
            ) {
                result.diagnostics.push(d);
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
                    rfc_path_display.clone(),
                ));
            } else if let Err(d) = check_ref_hierarchy(
                &rfc.rfc.rfc_id,
                supersedes,
                &rfc_path_display,
                ReferenceSurface::StructuredRef,
            ) {
                result.diagnostics.push(d);
            }
        }
    }

    // Validate ADR refs
    for adr in &index.adrs {
        let adr_path_display = config.display_path(&adr.path).display().to_string();
        for ref_id in &adr.meta().refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0304AdrRefNotFound,
                    format!(
                        "ADR '{}' references unknown artifact: {}",
                        adr.meta().id,
                        ref_id
                    ),
                    adr_path_display.clone(),
                ));
            } else if let Err(d) = check_ref_hierarchy(
                &adr.meta().id,
                ref_id,
                &adr_path_display,
                ReferenceSurface::StructuredRef,
            ) {
                result.diagnostics.push(d);
            }
        }
    }

    // Validate Work Item refs
    for work in &index.work_items {
        let work_path_display = config.display_path(&work.path).display().to_string();
        for ref_id in &work.meta().refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0404WorkRefNotFound,
                    format!(
                        "Work item '{}' references unknown artifact: {}",
                        work.meta().id,
                        ref_id
                    ),
                    work_path_display.clone(),
                ));
            }
        }
    }
}
