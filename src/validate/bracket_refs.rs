use super::ValidationResult;
use super::reference_hierarchy::{ReferenceSurface, check_ref_hierarchy};
use crate::artifact_index::artifact_ref_ids;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrStatus, ProjectIndex, RfcStatus, WorkItemStatus};
use regex::Regex;
use std::collections::HashSet;

const BARE_ARTIFACT_ID_PATTERN: &str = r"\b(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3})|CONF-[A-Z][A-Z0-9-]*)\b";

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

#[derive(Clone, Copy)]
struct ScanPolicy {
    scan_bare_text: bool,
    warn_on_bare_text: bool,
}

struct GovernedTextSource<'a> {
    text: &'a str,
    owner_id: &'a str,
    path: String,
    field: String,
    policy: ScanPolicy,
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

    for source in governed_text_sources(index, config) {
        scan_reference_hierarchy(
            &scanner,
            source.text,
            source.owner_id,
            TextSource {
                path: &source.path,
                field: &source.field,
            },
            source.policy,
            result,
        );
    }
}

fn governed_text_sources<'a>(
    index: &'a ProjectIndex,
    config: &Config,
) -> Vec<GovernedTextSource<'a>> {
    let mut sources = Vec::new();
    collect_rfc_text_sources(index, config, &mut sources);
    collect_adr_text_sources(index, config, &mut sources);
    collect_work_text_sources(index, config, &mut sources);
    sources
}

fn collect_rfc_text_sources<'a>(
    index: &'a ProjectIndex,
    config: &Config,
    sources: &mut Vec<GovernedTextSource<'a>>,
) {
    for rfc in &index.rfcs {
        let rfc_path = config.display_path(&rfc.path).display().to_string();
        let rid = rfc.rfc.rfc_id.as_str();
        let warn_on_bare_text = rfc.rfc.status == RfcStatus::Draft;
        for clause in &rfc.clauses {
            sources.push(GovernedTextSource {
                text: &clause.spec.text,
                owner_id: rid,
                path: config.display_path(&clause.path).display().to_string(),
                field: format!("{} content.text", clause.spec.clause_id),
                policy: ScanPolicy {
                    scan_bare_text: true,
                    warn_on_bare_text,
                },
            });
        }
        for (entry_index, entry) in rfc.rfc.changelog.iter().enumerate() {
            if let Some(ref notes) = entry.notes {
                sources.push(GovernedTextSource {
                    text: notes,
                    owner_id: rid,
                    path: rfc_path.clone(),
                    field: format!("changelog[{entry_index}].notes"),
                    policy: ScanPolicy {
                        scan_bare_text: false,
                        warn_on_bare_text: false,
                    },
                });
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
                    sources.push(GovernedTextSource {
                        text: line,
                        owner_id: rid,
                        path: rfc_path.clone(),
                        field: format!("changelog[{entry_index}].{section}[{line_index}]"),
                        policy: ScanPolicy {
                            scan_bare_text: false,
                            warn_on_bare_text: false,
                        },
                    });
                }
            }
        }
    }
}

fn collect_adr_text_sources<'a>(
    index: &'a ProjectIndex,
    config: &Config,
    sources: &mut Vec<GovernedTextSource<'a>>,
) {
    for adr in &index.adrs {
        let adr_path = config.display_path(&adr.path).display().to_string();
        let aid = adr.meta().id.as_str();
        let warn_on_bare_text = adr.meta().status == AdrStatus::Proposed;
        let c = &adr.spec.content;
        let policy = ScanPolicy {
            scan_bare_text: true,
            warn_on_bare_text,
        };
        sources.extend([
            GovernedTextSource {
                text: &c.context,
                owner_id: aid,
                path: adr_path.clone(),
                field: "content.context".to_string(),
                policy,
            },
            GovernedTextSource {
                text: &c.decision,
                owner_id: aid,
                path: adr_path.clone(),
                field: "content.decision".to_string(),
                policy,
            },
            GovernedTextSource {
                text: &c.consequences,
                owner_id: aid,
                path: adr_path.clone(),
                field: "content.consequences".to_string(),
                policy,
            },
        ]);
        for (alt_index, alt) in c.alternatives.iter().enumerate() {
            sources.push(GovernedTextSource {
                text: &alt.text,
                owner_id: aid,
                path: adr_path.clone(),
                field: format!("content.alternatives[{alt_index}].text"),
                policy,
            });
            for (pro_index, p) in alt.pros.iter().enumerate() {
                sources.push(GovernedTextSource {
                    text: p,
                    owner_id: aid,
                    path: adr_path.clone(),
                    field: format!("content.alternatives[{alt_index}].pros[{pro_index}]"),
                    policy,
                });
            }
            for (con_index, cons) in alt.cons.iter().enumerate() {
                sources.push(GovernedTextSource {
                    text: cons,
                    owner_id: aid,
                    path: adr_path.clone(),
                    field: format!("content.alternatives[{alt_index}].cons[{con_index}]"),
                    policy,
                });
            }
            if let Some(ref rr) = alt.rejection_reason {
                sources.push(GovernedTextSource {
                    text: rr,
                    owner_id: aid,
                    path: adr_path.clone(),
                    field: format!("content.alternatives[{alt_index}].rejection_reason"),
                    policy,
                });
            }
        }
    }
}

fn collect_work_text_sources<'a>(
    index: &'a ProjectIndex,
    config: &Config,
    sources: &mut Vec<GovernedTextSource<'a>>,
) {
    for work in &index.work_items {
        let work_path = config.display_path(&work.path).display().to_string();
        let wid = work.meta().id.as_str();
        let policy = ScanPolicy {
            scan_bare_text: true,
            warn_on_bare_text: work.meta().status != WorkItemStatus::Done,
        };
        let content = &work.spec.content;
        sources.push(GovernedTextSource {
            text: &content.description,
            owner_id: wid,
            path: work_path.clone(),
            field: "content.description".to_string(),
            policy,
        });
        for (criterion_index, criterion) in content.acceptance_criteria.iter().enumerate() {
            sources.push(GovernedTextSource {
                text: &criterion.text,
                owner_id: wid,
                path: work_path.clone(),
                field: format!("content.acceptance_criteria[{criterion_index}].text"),
                policy,
            });
        }
        for (note_index, note) in content.notes.iter().enumerate() {
            sources.push(GovernedTextSource {
                text: note,
                owner_id: wid,
                path: work_path.clone(),
                field: format!("content.notes[{note_index}]"),
                policy,
            });
        }
    }
}

fn scan_reference_hierarchy(
    scanner: &ReferenceScanner,
    text: &str,
    owner_id: &str,
    source: TextSource<'_>,
    policy: ScanPolicy,
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

    if !policy.scan_bare_text {
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
            Ok(()) if policy.warn_on_bare_text => result.diagnostics.push(
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
            ScanPolicy {
                scan_bare_text: true,
                warn_on_bare_text: true,
            },
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
            ScanPolicy {
                scan_bare_text: true,
                warn_on_bare_text: true,
            },
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
            ScanPolicy {
                scan_bare_text: true,
                warn_on_bare_text: true,
            },
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
            ScanPolicy {
                scan_bare_text: true,
                warn_on_bare_text: true,
            },
            &mut result,
        );

        assert!(result.diagnostics.is_empty());
        Ok(())
    }
}
