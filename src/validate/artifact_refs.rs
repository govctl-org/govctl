use super::ValidationResult;
use super::reference_hierarchy::{ReferenceSurface, check_ref_hierarchy};
use crate::artifact_index::artifact_ref_ids;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::ProjectIndex;
use std::collections::HashSet;

/// Validate refs fields in RFCs, ADRs and Work Items
pub(super) fn validate_artifact_refs(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let known_ids = artifact_ref_ids(index);

    // Validate RFC refs and supersedes
    for rfc in &index.rfcs {
        let rfc_path_display = config.display_path(&rfc.path).display().to_string();
        let rfc_ref_check = RefCheck {
            known_ids: &known_ids,
            owner_id: &rfc.rfc.rfc_id,
            path_display: &rfc_path_display,
            unknown_code: DiagnosticCode::E0105RfcRefNotFound,
            check_hierarchy: true,
        };
        validate_refs(result, rfc_ref_check, &rfc.rfc.refs, |ref_id| {
            format!(
                "RFC '{}' references unknown artifact: {}",
                rfc.rfc.rfc_id, ref_id
            )
        });

        // Validate supersedes field
        if let Some(ref supersedes) = rfc.rfc.supersedes {
            let supersedes_check = RefCheck {
                known_ids: &known_ids,
                owner_id: &rfc.rfc.rfc_id,
                path_display: &rfc_path_display,
                unknown_code: DiagnosticCode::E0106RfcSupersedesNotFound,
                check_hierarchy: true,
            };
            validate_refs(
                result,
                supersedes_check,
                std::slice::from_ref(supersedes),
                |ref_id| {
                    format!(
                        "RFC '{}' supersedes unknown RFC: {}",
                        rfc.rfc.rfc_id, ref_id
                    )
                },
            );
        }
    }

    // Validate ADR refs
    for adr in &index.adrs {
        let adr_path_display = config.display_path(&adr.path).display().to_string();
        let adr_ref_check = RefCheck {
            known_ids: &known_ids,
            owner_id: &adr.meta().id,
            path_display: &adr_path_display,
            unknown_code: DiagnosticCode::E0304AdrRefNotFound,
            check_hierarchy: true,
        };
        validate_refs(result, adr_ref_check, &adr.meta().refs, |ref_id| {
            format!(
                "ADR '{}' references unknown artifact: {}",
                adr.meta().id,
                ref_id
            )
        });
    }

    // Validate Work Item refs
    for work in &index.work_items {
        let work_path_display = config.display_path(&work.path).display().to_string();
        let work_ref_check = RefCheck {
            known_ids: &known_ids,
            owner_id: &work.meta().id,
            path_display: &work_path_display,
            unknown_code: DiagnosticCode::E0404WorkRefNotFound,
            check_hierarchy: false,
        };
        validate_refs(result, work_ref_check, &work.meta().refs, |ref_id| {
            format!(
                "Work item '{}' references unknown artifact: {}",
                work.meta().id,
                ref_id
            )
        });
    }
}

pub fn validate_artifact_ref_edit(
    config: &Config,
    owner_id: &str,
    ref_id: &str,
    diagnostic_path: &str,
) -> DiagnosticResult<()> {
    let index = crate::load::load_project(config).map_err(|mut diagnostics| {
        if diagnostics.is_empty() {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                "Failed to load project for refs validation",
                diagnostic_path,
            )
        } else {
            diagnostics.remove(0)
        }
    })?;
    let known_ids = artifact_ref_ids(&index);
    if !known_ids.contains(ref_id) {
        return Err(Diagnostic::new(
            unknown_ref_code(owner_id),
            unknown_ref_message(owner_id, ref_id),
            diagnostic_path,
        ));
    }
    check_ref_hierarchy(
        owner_id,
        ref_id,
        diagnostic_path,
        ReferenceSurface::StructuredRef,
    )
}

#[derive(Clone, Copy)]
struct RefCheck<'a> {
    known_ids: &'a HashSet<String>,
    owner_id: &'a str,
    path_display: &'a str,
    unknown_code: DiagnosticCode,
    check_hierarchy: bool,
}

fn validate_refs<'a, I, F>(
    result: &mut ValidationResult,
    check: RefCheck<'_>,
    refs: I,
    unknown_message: F,
) where
    I: IntoIterator<Item = &'a String>,
    F: Fn(&str) -> String,
{
    for ref_id in refs {
        if !check.known_ids.contains(ref_id) {
            result.diagnostics.push(Diagnostic::new(
                check.unknown_code,
                unknown_message(ref_id),
                check.path_display.to_string(),
            ));
        } else if check.check_hierarchy
            && let Err(diagnostic) = check_ref_hierarchy(
                check.owner_id,
                ref_id,
                check.path_display,
                ReferenceSurface::StructuredRef,
            )
        {
            result.diagnostics.push(diagnostic);
        }
    }
}

fn unknown_ref_code(owner_id: &str) -> DiagnosticCode {
    if owner_id.starts_with("RFC-") {
        DiagnosticCode::E0105RfcRefNotFound
    } else if owner_id.starts_with("ADR-") {
        DiagnosticCode::E0304AdrRefNotFound
    } else {
        DiagnosticCode::E0404WorkRefNotFound
    }
}

fn unknown_ref_message(owner_id: &str, ref_id: &str) -> String {
    if owner_id.starts_with("RFC-") {
        format!("RFC '{owner_id}' references unknown artifact: {ref_id}")
    } else if owner_id.starts_with("ADR-") {
        format!("ADR '{owner_id}' references unknown artifact: {ref_id}")
    } else {
        format!("Work item '{owner_id}' references unknown artifact: {ref_id}")
    }
}
