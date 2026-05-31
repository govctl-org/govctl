use super::ValidationResult;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;
use regex::Regex;

/// Validate `[[...]]` link targets in RFC and ADR content per [[RFC-0000:C-REFERENCE-HIERARCHY]].
pub(super) fn validate_bracket_reference_hierarchy(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let re = match Regex::new(&config.source_scan.pattern) {
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

    for rfc in &index.rfcs {
        let rfc_path = config.display_path(&rfc.path).display().to_string();
        let rid = rfc.rfc.rfc_id.as_str();
        for clause in &rfc.clauses {
            let clause_path = config.display_path(&clause.path).display().to_string();
            scan_rfc_bracket_refs(&re, &clause.spec.text, rid, &clause_path, result);
        }
        for entry in &rfc.rfc.changelog {
            if let Some(ref notes) = entry.notes {
                scan_rfc_bracket_refs(&re, notes, rid, &rfc_path, result);
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
                scan_rfc_bracket_refs(&re, line, rid, &rfc_path, result);
            }
        }
    }

    for adr in &index.adrs {
        let adr_path = config.display_path(&adr.path).display().to_string();
        let aid = adr.meta().id.as_str();
        let c = &adr.spec.content;
        scan_adr_bracket_refs(&re, &c.context, aid, &adr_path, result);
        scan_adr_bracket_refs(&re, &c.decision, aid, &adr_path, result);
        scan_adr_bracket_refs(&re, &c.consequences, aid, &adr_path, result);
        for alt in &c.alternatives {
            scan_adr_bracket_refs(&re, &alt.text, aid, &adr_path, result);
            for p in &alt.pros {
                scan_adr_bracket_refs(&re, p, aid, &adr_path, result);
            }
            for cons in &alt.cons {
                scan_adr_bracket_refs(&re, cons, aid, &adr_path, result);
            }
            if let Some(ref rr) = alt.rejection_reason {
                scan_adr_bracket_refs(&re, rr, aid, &adr_path, result);
            }
        }
    }
}

fn scan_rfc_bracket_refs(
    re: &Regex,
    text: &str,
    rfc_id: &str,
    path: &str,
    result: &mut ValidationResult,
) {
    for caps in re.captures_iter(text) {
        let Some(m) = caps.get(1) else {
            continue;
        };
        let target = m.as_str();
        if target.starts_with("ADR-") || target.starts_with("WI-") {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0112RfcReferenceHierarchy,
                format!(
                    "RFC '{rfc_id}' links to [[{target}]], but RFCs are higher authority than ADRs and Work Items — remove this link (the ADR or Work Item should reference the RFC, not the other way around)"
                ),
                path,
            ));
        }
    }
}

fn scan_adr_bracket_refs(
    re: &Regex,
    text: &str,
    adr_id: &str,
    path: &str,
    result: &mut ValidationResult,
) {
    for caps in re.captures_iter(text) {
        let Some(m) = caps.get(1) else {
            continue;
        };
        let target = m.as_str();
        if target.starts_with("WI-") {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0306AdrReferenceHierarchy,
                format!("ADR '{adr_id}' links to [[{target}]], but ADRs are higher authority than Work Items — remove this link (the Work Item should reference the ADR, not the other way around)"),
                path,
            ));
        }
    }
}
