use super::ValidationResult;
use super::reference_hierarchy::{ReferenceSurface, check_ref_hierarchy};
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
        if let Err(diagnostic) =
            check_ref_hierarchy(rfc_id, target, path, ReferenceSurface::BracketLink)
        {
            result.diagnostics.push(diagnostic);
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
        if let Err(diagnostic) =
            check_ref_hierarchy(adr_id, target, path, ReferenceSurface::BracketLink)
        {
            result.diagnostics.push(diagnostic);
        }
    }
}
