use crate::diagnostic::{Diagnostic, DiagnosticCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReferenceSurface {
    StructuredRef,
    BracketLink,
}

/// Enforce [[RFC-0000:C-REFERENCE-HIERARCHY]] across refs and inline links.
pub(super) fn check_ref_hierarchy(
    owner_id: &str,
    target_id: &str,
    diagnostic_path: &str,
    surface: ReferenceSurface,
) -> Result<(), Diagnostic> {
    let owner_is_rfc = owner_id.starts_with("RFC-");
    let owner_is_adr = owner_id.starts_with("ADR-");
    let owner_is_wi = owner_id.starts_with("WI-");

    if owner_is_wi {
        return Ok(());
    }
    if owner_is_rfc && (target_id.starts_with("ADR-") || target_id.starts_with("WI-")) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0112RfcReferenceHierarchy,
            hierarchy_message("RFC", owner_id, target_id, surface),
            diagnostic_path,
        ));
    }
    if owner_is_adr && target_id.starts_with("WI-") {
        return Err(Diagnostic::new(
            DiagnosticCode::E0306AdrReferenceHierarchy,
            hierarchy_message("ADR", owner_id, target_id, surface),
            diagnostic_path,
        ));
    }
    Ok(())
}

fn hierarchy_message(
    owner_kind: &str,
    owner_id: &str,
    target_id: &str,
    surface: ReferenceSurface,
) -> String {
    match (owner_kind, surface) {
        ("RFC", ReferenceSurface::StructuredRef) => format!(
            "RFC '{owner_id}' references {target_id}, but RFCs are higher authority than ADRs and Work Items — remove this reference (the ADR or Work Item should reference the RFC, not the other way around)"
        ),
        ("RFC", ReferenceSurface::BracketLink) => format!(
            "RFC '{owner_id}' links to [[{target_id}]], but RFCs are higher authority than ADRs and Work Items — remove this link (the ADR or Work Item should reference the RFC, not the other way around)"
        ),
        ("ADR", ReferenceSurface::StructuredRef) => format!(
            "ADR '{owner_id}' references {target_id}, but ADRs are higher authority than Work Items — remove this reference (the Work Item should reference the ADR, not the other way around)"
        ),
        ("ADR", ReferenceSurface::BracketLink) => format!(
            "ADR '{owner_id}' links to [[{target_id}]], but ADRs are higher authority than Work Items — remove this link (the Work Item should reference the ADR, not the other way around)"
        ),
        _ => unreachable!("only RFC and ADR hierarchy violations are diagnosable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_rejects_adr_and_wi() {
        assert!(
            check_ref_hierarchy("RFC-0001", "ADR-0001", "f", ReferenceSurface::StructuredRef,)
                .is_err()
        );
        assert!(
            check_ref_hierarchy(
                "RFC-0001",
                "WI-2026-01-17-001",
                "f",
                ReferenceSurface::StructuredRef,
            )
            .is_err()
        );
    }

    #[test]
    fn rfc_allows_rfc_and_clause() {
        assert!(
            check_ref_hierarchy("RFC-0001", "RFC-0002", "f", ReferenceSurface::StructuredRef,)
                .is_ok()
        );
        assert!(
            check_ref_hierarchy(
                "RFC-0001",
                "RFC-0002:C-FOO",
                "f",
                ReferenceSurface::StructuredRef,
            )
            .is_ok()
        );
    }

    #[test]
    fn adr_rejects_wi() {
        assert!(
            check_ref_hierarchy(
                "ADR-0001",
                "WI-2026-01-17-001",
                "f",
                ReferenceSurface::StructuredRef,
            )
            .is_err()
        );
    }

    #[test]
    fn adr_allows_rfc_adr() {
        assert!(
            check_ref_hierarchy(
                "ADR-0001",
                "RFC-0000:C-RFC-DEF",
                "f",
                ReferenceSurface::StructuredRef,
            )
            .is_ok()
        );
        assert!(
            check_ref_hierarchy("ADR-0001", "ADR-0002", "f", ReferenceSurface::StructuredRef,)
                .is_ok()
        );
    }

    #[test]
    fn work_allows_any() {
        assert!(
            check_ref_hierarchy(
                "WI-2026-01-17-001",
                "WI-2026-01-17-002",
                "f",
                ReferenceSurface::StructuredRef,
            )
            .is_ok()
        );
        assert!(
            check_ref_hierarchy(
                "WI-2026-01-17-001",
                "ADR-0001",
                "f",
                ReferenceSurface::StructuredRef,
            )
            .is_ok()
        );
    }

    #[test]
    fn preserves_structured_ref_diagnostic_wording() {
        let result =
            check_ref_hierarchy("RFC-0001", "ADR-0001", "f", ReferenceSurface::StructuredRef);

        assert!(result.is_err(), "RFC to ADR structured ref should fail");
        if let Err(err) = result {
            assert_eq!(
                err.message,
                "RFC 'RFC-0001' references ADR-0001, but RFCs are higher authority than ADRs and Work Items — remove this reference (the ADR or Work Item should reference the RFC, not the other way around)"
            );
        }
    }

    #[test]
    fn preserves_bracket_link_diagnostic_wording() {
        let target_id = "WI-2026-01-17-001";
        let result = check_ref_hierarchy("ADR-0001", target_id, "f", ReferenceSurface::BracketLink);
        assert!(result.is_err(), "ADR to WI bracket link should fail");
        if let Err(err) = result {
            assert_eq!(
                err.message,
                format!(
                    "ADR 'ADR-0001' links to [[{target_id}]], but ADRs are higher authority than Work Items — remove this link (the Work Item should reference the ADR, not the other way around)"
                )
            );
        }
    }
}
