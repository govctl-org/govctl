use super::ValidationResult;
use super::reference_hierarchy::{ReferenceSurface, check_ref_hierarchy};
use crate::artifact_index::artifact_ref_ids;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;
use regex::Regex;
use std::collections::HashSet;

const BARE_ARTIFACT_ID_PATTERN: &str = r"\b(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3}))\b";

struct ReferenceScanner {
    bracket_re: Regex,
    bare_re: Regex,
    known_ids: HashSet<String>,
}

/// Validate references in RFC and ADR content per [[RFC-0000:C-REFERENCE-HIERARCHY]].
pub(super) fn validate_bracket_reference_hierarchy(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let bracket_re = match Regex::new(&config.source_scan.pattern) {
        Ok(r) => r,
        Err(e) => {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("Invalid source_scan.pattern for bracket reference scan: {e}"),
                "gov/config.toml".to_string(),
            ));
            return;
        }
    };
    let bare_re = match Regex::new(BARE_ARTIFACT_ID_PATTERN) {
        Ok(r) => r,
        Err(e) => {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                format!("Invalid built-in bare artifact reference scan pattern: {e}"),
                "internal",
            ));
            return;
        }
    };
    let scanner = ReferenceScanner {
        bracket_re,
        bare_re,
        known_ids: artifact_ref_ids(index),
    };

    for rfc in &index.rfcs {
        let rfc_path = config.display_path(&rfc.path).display().to_string();
        let rid = rfc.rfc.rfc_id.as_str();
        for clause in &rfc.clauses {
            let clause_path = config.display_path(&clause.path).display().to_string();
            scan_rfc_reference_hierarchy(
                &scanner,
                &clause.spec.text,
                rid,
                &clause_path,
                true,
                result,
            );
        }
        for entry in &rfc.rfc.changelog {
            if let Some(ref notes) = entry.notes {
                scan_rfc_reference_hierarchy(&scanner, notes, rid, &rfc_path, false, result);
            }
            for line in entry
                .added
                .iter()
                .chain(entry.changed.iter())
                .chain(entry.deprecated.iter())
                .chain(entry.removed.iter())
                .chain(entry.fixed.iter())
                .chain(entry.security.iter())
            {
                scan_rfc_reference_hierarchy(&scanner, line, rid, &rfc_path, false, result);
            }
        }
    }

    for adr in &index.adrs {
        let adr_path = config.display_path(&adr.path).display().to_string();
        let aid = adr.meta().id.as_str();
        let c = &adr.spec.content;
        scan_adr_reference_hierarchy(&scanner, &c.context, aid, &adr_path, result);
        scan_adr_reference_hierarchy(&scanner, &c.decision, aid, &adr_path, result);
        scan_adr_reference_hierarchy(&scanner, &c.consequences, aid, &adr_path, result);
        for alt in &c.alternatives {
            scan_adr_reference_hierarchy(&scanner, &alt.text, aid, &adr_path, result);
            for p in &alt.pros {
                scan_adr_reference_hierarchy(&scanner, p, aid, &adr_path, result);
            }
            for cons in &alt.cons {
                scan_adr_reference_hierarchy(&scanner, cons, aid, &adr_path, result);
            }
            if let Some(ref rr) = alt.rejection_reason {
                scan_adr_reference_hierarchy(&scanner, rr, aid, &adr_path, result);
            }
        }
    }
}

fn scan_rfc_reference_hierarchy(
    scanner: &ReferenceScanner,
    text: &str,
    rfc_id: &str,
    path: &str,
    scan_bare_text: bool,
    result: &mut ValidationResult,
) {
    scan_reference_hierarchy(scanner, text, rfc_id, path, scan_bare_text, result);
}

fn scan_adr_reference_hierarchy(
    scanner: &ReferenceScanner,
    text: &str,
    adr_id: &str,
    path: &str,
    result: &mut ValidationResult,
) {
    scan_reference_hierarchy(scanner, text, adr_id, path, true, result);
}

fn scan_reference_hierarchy(
    scanner: &ReferenceScanner,
    text: &str,
    owner_id: &str,
    path: &str,
    scan_bare_text: bool,
    result: &mut ValidationResult,
) {
    let mut bracket_ranges = Vec::new();
    for caps in scanner.bracket_re.captures_iter(text) {
        if let Some(full) = caps.get(0) {
            bracket_ranges.push(full.range());
        }
        let Some(m) = caps.get(1) else {
            continue;
        };
        let target = m.as_str();
        if let Err(diagnostic) =
            check_ref_hierarchy(owner_id, target, path, ReferenceSurface::BracketLink)
        {
            result.diagnostics.push(diagnostic);
        }
    }

    if !scan_bare_text {
        return;
    }

    for caps in scanner.bare_re.captures_iter(text) {
        let Some(m) = caps.get(1) else {
            continue;
        };
        if bracket_ranges
            .iter()
            .any(|range| range.start <= m.start() && m.end() <= range.end)
        {
            continue;
        }
        let target = m.as_str();
        if !scanner.known_ids.contains(target) {
            continue;
        }
        if let Err(diagnostic) =
            check_ref_hierarchy(owner_id, target, path, ReferenceSurface::BareText)
        {
            result.diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::DiagnosticResult;

    fn bracket_re() -> DiagnosticResult<Regex> {
        Regex::new(
            r"\[\[(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3}))\]\]",
        )
        .map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                format!("test bracket regex must compile: {err}"),
                "test",
            )
        })
    }

    fn bare_re() -> DiagnosticResult<Regex> {
        Regex::new(BARE_ARTIFACT_ID_PATTERN).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                format!("test bare regex must compile: {err}"),
                "test",
            )
        })
    }

    fn scanner(known_ids: HashSet<String>) -> DiagnosticResult<ReferenceScanner> {
        Ok(ReferenceScanner {
            bracket_re: bracket_re()?,
            bare_re: bare_re()?,
            known_ids,
        })
    }

    #[test]
    fn bare_known_adr_in_rfc_text_violates_hierarchy() -> DiagnosticResult<()> {
        let mut known_ids = HashSet::new();
        known_ids.insert("ADR-0001".to_string());
        let mut result = ValidationResult::default();

        scan_reference_hierarchy(
            &scanner(known_ids)?,
            "This mentions ADR-0001 without brackets.",
            "RFC-0001",
            "f",
            true,
            &mut result,
        );

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].code,
            DiagnosticCode::E0112RfcReferenceHierarchy
        );
        Ok(())
    }

    #[test]
    fn bare_unknown_artifact_shape_in_rfc_text_is_not_a_reference() -> DiagnosticResult<()> {
        let known_ids = HashSet::new();
        let mut result = ValidationResult::default();

        scan_reference_hierarchy(
            &scanner(known_ids)?,
            "This mentions ADR-0001 only as an example shape.",
            "RFC-0001",
            "f",
            true,
            &mut result,
        );

        assert!(result.diagnostics.is_empty());
        Ok(())
    }
}
