use super::ValidationResult;
use super::reference_hierarchy::{ReferenceSurface, check_ref_hierarchy};
use crate::artifact_index::artifact_ref_ids;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrStatus, ProjectIndex, RfcStatus, WorkItemStatus};
use regex::Regex;
use std::collections::HashSet;

const BARE_ARTIFACT_ID_PATTERN: &str = r"\b(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3}))\b";

struct ReferenceScanner {
    bracket_re: Regex,
    bare_re: Regex,
    known_ids: HashSet<String>,
}

#[derive(Clone, Copy)]
struct TextSource<'a> {
    path: &'a str,
    field: &'a str,
}

/// Validate inline references in governed prose per [[RFC-0000:C-REFERENCE-HIERARCHY]].
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
        let warn_on_bare_text = rfc.rfc.status == RfcStatus::Draft;
        for clause in &rfc.clauses {
            let clause_path = config.display_path(&clause.path).display().to_string();
            let field = format!("{} content.text", clause.spec.clause_id);
            scan_rfc_reference_hierarchy(
                &scanner,
                &clause.spec.text,
                rid,
                TextSource {
                    path: &clause_path,
                    field: &field,
                },
                true,
                warn_on_bare_text,
                result,
            );
        }
        for (entry_index, entry) in rfc.rfc.changelog.iter().enumerate() {
            if let Some(ref notes) = entry.notes {
                let field = format!("changelog[{entry_index}].notes");
                scan_rfc_reference_hierarchy(
                    &scanner,
                    notes,
                    rid,
                    TextSource {
                        path: &rfc_path,
                        field: &field,
                    },
                    false,
                    false,
                    result,
                );
            }
            let changelog_sections = [
                ("added", &entry.added),
                ("changed", &entry.changed),
                ("deprecated", &entry.deprecated),
                ("removed", &entry.removed),
                ("fixed", &entry.fixed),
                ("security", &entry.security),
            ];
            for (section, lines) in changelog_sections {
                for (line_index, line) in lines.iter().enumerate() {
                    let field = format!("changelog[{entry_index}].{section}[{line_index}]");
                    scan_rfc_reference_hierarchy(
                        &scanner,
                        line,
                        rid,
                        TextSource {
                            path: &rfc_path,
                            field: &field,
                        },
                        false,
                        false,
                        result,
                    );
                }
            }
        }
    }

    for adr in &index.adrs {
        let adr_path = config.display_path(&adr.path).display().to_string();
        let aid = adr.meta().id.as_str();
        let warn_on_bare_text = adr.meta().status == AdrStatus::Proposed;
        let c = &adr.spec.content;
        scan_adr_reference_hierarchy(
            &scanner,
            &c.context,
            aid,
            TextSource {
                path: &adr_path,
                field: "content.context",
            },
            warn_on_bare_text,
            result,
        );
        scan_adr_reference_hierarchy(
            &scanner,
            &c.decision,
            aid,
            TextSource {
                path: &adr_path,
                field: "content.decision",
            },
            warn_on_bare_text,
            result,
        );
        scan_adr_reference_hierarchy(
            &scanner,
            &c.consequences,
            aid,
            TextSource {
                path: &adr_path,
                field: "content.consequences",
            },
            warn_on_bare_text,
            result,
        );
        for (alt_index, alt) in c.alternatives.iter().enumerate() {
            let alt_text_field = format!("content.alternatives[{alt_index}].text");
            scan_adr_reference_hierarchy(
                &scanner,
                &alt.text,
                aid,
                TextSource {
                    path: &adr_path,
                    field: &alt_text_field,
                },
                warn_on_bare_text,
                result,
            );
            for (pro_index, p) in alt.pros.iter().enumerate() {
                let pro_field = format!("content.alternatives[{alt_index}].pros[{pro_index}]");
                scan_adr_reference_hierarchy(
                    &scanner,
                    p,
                    aid,
                    TextSource {
                        path: &adr_path,
                        field: &pro_field,
                    },
                    warn_on_bare_text,
                    result,
                );
            }
            for (con_index, cons) in alt.cons.iter().enumerate() {
                let con_field = format!("content.alternatives[{alt_index}].cons[{con_index}]");
                scan_adr_reference_hierarchy(
                    &scanner,
                    cons,
                    aid,
                    TextSource {
                        path: &adr_path,
                        field: &con_field,
                    },
                    warn_on_bare_text,
                    result,
                );
            }
            if let Some(ref rr) = alt.rejection_reason {
                let rejection_field = format!("content.alternatives[{alt_index}].rejection_reason");
                scan_adr_reference_hierarchy(
                    &scanner,
                    rr,
                    aid,
                    TextSource {
                        path: &adr_path,
                        field: &rejection_field,
                    },
                    warn_on_bare_text,
                    result,
                );
            }
        }
    }

    for work in &index.work_items {
        let work_path = config.display_path(&work.path).display().to_string();
        let wid = work.meta().id.as_str();
        let warn_on_bare_text = work.meta().status != WorkItemStatus::Done;
        let content = &work.spec.content;
        scan_work_reference_syntax(
            &scanner,
            &content.description,
            wid,
            TextSource {
                path: &work_path,
                field: "content.description",
            },
            warn_on_bare_text,
            result,
        );
        for (criterion_index, criterion) in content.acceptance_criteria.iter().enumerate() {
            let criterion_field = format!("content.acceptance_criteria[{criterion_index}].text");
            scan_work_reference_syntax(
                &scanner,
                &criterion.text,
                wid,
                TextSource {
                    path: &work_path,
                    field: &criterion_field,
                },
                warn_on_bare_text,
                result,
            );
        }
        for (note_index, note) in content.notes.iter().enumerate() {
            let note_field = format!("content.notes[{note_index}]");
            scan_work_reference_syntax(
                &scanner,
                note,
                wid,
                TextSource {
                    path: &work_path,
                    field: &note_field,
                },
                warn_on_bare_text,
                result,
            );
        }
    }
}

fn scan_rfc_reference_hierarchy(
    scanner: &ReferenceScanner,
    text: &str,
    rfc_id: &str,
    source: TextSource<'_>,
    scan_bare_text: bool,
    warn_on_bare_text: bool,
    result: &mut ValidationResult,
) {
    scan_reference_hierarchy(
        scanner,
        text,
        rfc_id,
        source,
        scan_bare_text,
        warn_on_bare_text,
        result,
    );
}

fn scan_adr_reference_hierarchy(
    scanner: &ReferenceScanner,
    text: &str,
    adr_id: &str,
    source: TextSource<'_>,
    warn_on_bare_text: bool,
    result: &mut ValidationResult,
) {
    scan_reference_hierarchy(
        scanner,
        text,
        adr_id,
        source,
        true,
        warn_on_bare_text,
        result,
    );
}

fn scan_work_reference_syntax(
    scanner: &ReferenceScanner,
    text: &str,
    work_id: &str,
    source: TextSource<'_>,
    warn_on_bare_text: bool,
    result: &mut ValidationResult,
) {
    scan_reference_hierarchy(
        scanner,
        text,
        work_id,
        source,
        true,
        warn_on_bare_text,
        result,
    );
}

fn scan_reference_hierarchy(
    scanner: &ReferenceScanner,
    text: &str,
    owner_id: &str,
    source: TextSource<'_>,
    scan_bare_text: bool,
    warn_on_bare_text: bool,
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
            check_ref_hierarchy(owner_id, target, source.path, ReferenceSurface::BracketLink)
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
        match check_ref_hierarchy(owner_id, target, source.path, ReferenceSurface::BareText) {
            Ok(()) if warn_on_bare_text => result.diagnostics.push(
                bare_artifact_reference_warning(owner_id, target, source, text, m.start()),
            ),
            Ok(()) => {}
            Err(diagnostic) => result.diagnostics.push(diagnostic),
        }
    }
}

fn bare_artifact_reference_warning(
    owner_id: &str,
    target: &str,
    source: TextSource<'_>,
    text: &str,
    match_start: usize,
) -> Diagnostic {
    let (line, context) = source_line_context(text, match_start);
    Diagnostic::new(
        DiagnosticCode::W0112BareArtifactReference,
        format!(
            "Artifact '{owner_id}' {field} line {line} mentions known artifact ID {target} without [[...]] inline reference syntax (hint: use [[{target}]]; context: \"{context}\")",
            field = source.field,
        ),
        source.path,
    )
}

fn source_line_context(text: &str, byte_offset: usize) -> (usize, String) {
    let line = text[..byte_offset].bytes().filter(|b| *b == b'\n').count() + 1;
    let line_start = text[..byte_offset].rfind('\n').map_or(0, |idx| idx + 1);
    let line_end = text[byte_offset..]
        .find('\n')
        .map_or(text.len(), |idx| byte_offset + idx);
    let context = collapse_context_whitespace(&text[line_start..line_end]);
    (line, truncate_context(&context))
}

fn collapse_context_whitespace(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_context(context: &str) -> String {
    const MAX_CONTEXT_CHARS: usize = 120;
    let mut out = String::new();
    for (count, ch) in context.chars().enumerate() {
        if count == MAX_CONTEXT_CHARS {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out.replace('"', "\\\"")
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
            TextSource {
                path: "f",
                field: "content.text",
            },
            true,
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
            TextSource {
                path: "f",
                field: "content.text",
            },
            true,
            true,
            &mut result,
        );

        assert!(result.diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn bare_known_rfc_in_adr_text_warns() -> DiagnosticResult<()> {
        let mut known_ids = HashSet::new();
        known_ids.insert("RFC-0001".to_string());
        let mut result = ValidationResult::default();

        scan_reference_hierarchy(
            &scanner(known_ids)?,
            "Intro line.\nThis follows RFC-0001.",
            "ADR-0001",
            TextSource {
                path: "f",
                field: "content.decision",
            },
            true,
            true,
            &mut result,
        );

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].code,
            DiagnosticCode::W0112BareArtifactReference
        );
        assert!(
            result.diagnostics[0]
                .message
                .contains("content.decision line 2"),
            "message: {}",
            result.diagnostics[0].message
        );
        assert!(
            result.diagnostics[0]
                .message
                .contains("context: \"This follows RFC-0001.\""),
            "message: {}",
            result.diagnostics[0].message
        );
        Ok(())
    }

    #[test]
    fn bracketed_known_rfc_in_adr_text_does_not_warn() -> DiagnosticResult<()> {
        let mut known_ids = HashSet::new();
        known_ids.insert("RFC-0001".to_string());
        let mut result = ValidationResult::default();

        scan_reference_hierarchy(
            &scanner(known_ids)?,
            "This follows [[RFC-0001]].",
            "ADR-0001",
            TextSource {
                path: "f",
                field: "content.decision",
            },
            true,
            true,
            &mut result,
        );

        assert!(result.diagnostics.is_empty());
        Ok(())
    }
}
