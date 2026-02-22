//! SSOT to Markdown rendering.
//!
//! Implements [[ADR-0003]] signatures and [[ADR-0011]] inline reference expansion.
//!
//! Rendered markdown files are read-only projections. Each includes:
//! - A "GENERATED" comment warning not to edit
//! - A SHA-256 signature for tampering detection
//! - Inline `[[artifact-id]]` references expanded to markdown links

use crate::config::Config;
use crate::model::{AdrEntry, ClauseKind, ClauseStatus, RfcIndex, WorkItemEntry};
use crate::signature::{
    compute_adr_signature, compute_rfc_signature, compute_work_item_signature,
    format_signature_header,
};
use crate::ui;
use regex::Regex;
use std::fmt::Write as FmtWrite;
use std::io::Write;

/// Generate a markdown link for an artifact reference.
///
/// Supports:
/// - RFC refs: `RFC-0000` → `[RFC-0000](../rfc/RFC-0000.md)`
/// - Clause refs: `RFC-0000:C-NAME` → `[RFC-0000:C-NAME](../rfc/RFC-0000.md#rfc-0000c-name)`
/// - ADR refs: `ADR-0001` → `[ADR-0001](../adr/ADR-0001.md)`
/// - Work Item refs: `WI-2026-01-17-001` → `[WI-2026-01-17-001](../work/WI-2026-01-17-001.md)`
fn ref_link(ref_id: &str) -> String {
    ref_link_with_base(ref_id, "..")
}

/// Generate a markdown link for an artifact reference from the repository root.
///
/// Used for files like CHANGELOG.md that live at the root level.
/// The `docs_output` path comes from config (e.g., "docs").
pub fn ref_link_from_root(ref_id: &str, docs_output: &str) -> String {
    ref_link_with_base(ref_id, docs_output)
}

/// Generate a markdown link with a configurable base path.
///
/// - `base`: Path prefix before `/rfc/`, `/adr/`, `/work/` (e.g., ".." or "docs")
fn ref_link_with_base(ref_id: &str, base: &str) -> String {
    if ref_id.starts_with("RFC-") {
        if ref_id.contains(':') {
            // Clause reference: RFC-0000:C-NAME
            let rfc_id = ref_id.split(':').next().unwrap_or(ref_id);
            // Anchor: lowercase, no special chars (GitHub-style slug)
            let anchor = ref_id.to_lowercase().replace(':', "");
            format!("[{}]({}/rfc/{}.md#{})", ref_id, base, rfc_id, anchor)
        } else {
            // RFC reference
            format!("[{}]({}/rfc/{}.md)", ref_id, base, ref_id)
        }
    } else if ref_id.starts_with("ADR-") {
        format!("[{}]({}/adr/{}.md)", ref_id, base, ref_id)
    } else if ref_id.starts_with("WI-") {
        format!("[{}]({}/work/{}.md)", ref_id, base, ref_id)
    } else {
        // Unknown type, return as-is
        ref_id.to_string()
    }
}

/// Render a list of refs as markdown links.
fn render_refs(refs: &[String]) -> String {
    refs.iter()
        .map(|r| ref_link(r))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Expand inline `[[artifact-id]]` references to markdown links.
///
/// Uses the pattern from source_scan config (per ADR-0011).
/// The pattern must have a capture group for the artifact ID.
pub fn expand_inline_refs(text: &str, pattern: &str) -> String {
    expand_inline_refs_with_linker(text, pattern, ref_link)
}

/// Expand inline `[[artifact-id]]` references to markdown links from repository root.
///
/// Used for files like CHANGELOG.md that live at the root level.
/// The `docs_output` path comes from config (e.g., "docs").
pub fn expand_inline_refs_from_root(text: &str, pattern: &str, docs_output: &str) -> String {
    expand_inline_refs_with_linker(text, pattern, |ref_id| {
        ref_link_from_root(ref_id, docs_output)
    })
}

/// Expand inline references using a custom link generator function.
fn expand_inline_refs_with_linker<F>(text: &str, pattern: &str, linker: F) -> String
where
    F: Fn(&str) -> String,
{
    let Ok(re) = Regex::new(pattern) else {
        // Invalid pattern, return text unchanged
        return text.to_string();
    };

    re.replace_all(text, |caps: &regex::Captures| {
        // Capture group 1 contains the artifact ID
        if let Some(artifact_id) = caps.get(1) {
            linker(artifact_id.as_str())
        } else {
            // No capture group, return match unchanged
            caps.get(0).map_or("", |m| m.as_str()).to_string()
        }
    })
    .to_string()
}

/// Indent continuation lines in multi-line text to preserve markdown list structure.
/// The first line is returned as-is; subsequent lines are prefixed with the indent.
fn indent_continuation(text: &str) -> String {
    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        return String::new();
    };
    let mut result = first.to_string();
    for line in lines {
        result.push('\n');
        result.push_str("  ");
        result.push_str(line);
    }
    result
}

/// Render an RFC to Markdown
///
/// # Errors
/// Returns an error if signature computation fails.
pub fn render_rfc(rfc: &RfcIndex) -> anyhow::Result<String> {
    let mut out = String::new();

    // Compute signature from source content (per ADR-0003)
    let signature = compute_rfc_signature(rfc)?;

    // Signature header (tampering detection per ADR-0003)
    out.push_str(&format_signature_header(&rfc.rfc.rfc_id, &signature));
    let _ = writeln!(out);

    // Title
    let _ = writeln!(out, "# {}: {}", rfc.rfc.rfc_id, rfc.rfc.title);
    let _ = writeln!(out);

    // Version info
    let _ = writeln!(
        out,
        "> **Version:** {} | **Status:** {} | **Phase:** {}",
        rfc.rfc.version,
        rfc.rfc.status.as_ref(),
        rfc.rfc.phase.as_ref()
    );
    let _ = writeln!(out);

    // References (expanded to markdown links)
    if !rfc.rfc.refs.is_empty() {
        let _ = writeln!(out, "**References:** {}", render_refs(&rfc.rfc.refs));
        let _ = writeln!(out);
    }

    // Render sections with clauses
    for (i, section) in rfc.rfc.sections.iter().enumerate() {
        let _ = writeln!(out, "---");
        let _ = writeln!(out);
        let _ = writeln!(out, "## {}. {}", i + 1, section.title);
        let _ = writeln!(out);

        // Find and render clauses for this section
        for clause_path in &section.clauses {
            if let Some(clause) = rfc.clauses.iter().find(|c| {
                c.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| clause_path.ends_with(n))
            }) {
                render_clause(&mut out, &rfc.rfc.rfc_id, clause);
            }
        }
    }

    // Changelog (Keep a Changelog format)
    if !rfc.rfc.changelog.is_empty() {
        let _ = writeln!(out, "---");
        let _ = writeln!(out);
        let _ = writeln!(out, "## Changelog");
        let _ = writeln!(out);

        for entry in &rfc.rfc.changelog {
            let _ = writeln!(out, "### v{} ({})", entry.version, entry.date);
            let _ = writeln!(out);

            if let Some(ref notes) = entry.notes {
                let _ = writeln!(out, "{notes}");
                let _ = writeln!(out);
            }

            render_changelog_section(&mut out, "Added", &entry.added);
            render_changelog_section(&mut out, "Changed", &entry.changed);
            render_changelog_section(&mut out, "Deprecated", &entry.deprecated);
            render_changelog_section(&mut out, "Removed", &entry.removed);
            render_changelog_section(&mut out, "Fixed", &entry.fixed);
            render_changelog_section(&mut out, "Security", &entry.security);
        }
    }

    Ok(out)
}

/// Render a changelog section (Keep a Changelog format)
fn render_changelog_section(out: &mut String, heading: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "#### {heading}");
    let _ = writeln!(out);
    for item in items {
        let _ = writeln!(out, "- {item}");
    }
    let _ = writeln!(out);
}

/// Generate anchor ID for a clause (matches ref_link anchor format).
fn clause_anchor(rfc_id: &str, clause_id: &str) -> String {
    format!("{}:{}", rfc_id, clause_id)
        .to_lowercase()
        .replace(':', "")
}

/// Render a single clause
fn render_clause(out: &mut String, rfc_id: &str, clause: &crate::model::ClauseEntry) {
    let spec = &clause.spec;

    // Clause header with ID anchor
    let kind_marker = match spec.kind {
        ClauseKind::Normative => "(Normative)",
        ClauseKind::Informative => "(Informative)",
    };

    // Generate anchor for clause linking (matches ref_link anchor format)
    let anchor = clause_anchor(rfc_id, &spec.clause_id);

    // Format title, wrapped in <del> if deprecated/superseded
    // Using HTML <del> instead of markdown ~~ avoids escaping issues with titles
    let title_part = format!("[{}:{}] {}", rfc_id, spec.clause_id, spec.title);
    let formatted_title = match spec.status {
        ClauseStatus::Active => title_part,
        ClauseStatus::Deprecated | ClauseStatus::Superseded => format!("<del>{}</del>", title_part),
    };

    let _ = writeln!(
        out,
        "### {} {kind_marker} <a id=\"{anchor}\"></a>",
        formatted_title
    );
    let _ = writeln!(out);

    // Clause text
    let _ = writeln!(out, "{}", spec.text);
    let _ = writeln!(out);

    // Superseded by notice
    if let Some(ref by) = spec.superseded_by {
        let _ = writeln!(out, "> **Superseded by:** {by}");
        let _ = writeln!(out);
    }

    // Since version
    if let Some(ref since) = spec.since {
        let _ = writeln!(out, "*Since: v{since}*");
        let _ = writeln!(out);
    }
}

/// Write rendered markdown to file with common formatting.
///
/// Handles dry-run preview, directory creation, and consistent formatting.
/// `preview_lines` controls how many lines to show in dry-run mode.
fn write_rendered_md(
    config: &Config,
    output_path: &std::path::Path,
    content: &str,
    dry_run: bool,
    preview_lines: usize,
) -> anyhow::Result<()> {
    // Trim trailing whitespace, ensure single trailing newline
    let content = format!("{}\n", content.trim_end());
    let display_path = config.display_path(output_path);

    if dry_run {
        ui::dry_run_preview(&display_path);
        for line in content.lines().take(preview_lines) {
            ui::preview_line(line);
        }
        ui::preview_truncated();
    } else {
        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(output_path)?;
        file.write_all(content.as_bytes())?;
        ui::rendered(&display_path);
    }

    Ok(())
}

/// Write rendered RFC to file
pub fn write_rfc(config: &Config, rfc: &RfcIndex, dry_run: bool) -> anyhow::Result<()> {
    let output_path = config.rfc_output().join(format!("{}.md", rfc.rfc.rfc_id));

    // Render and expand inline references (per ADR-0011)
    let raw = render_rfc(rfc)?;
    let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);

    write_rendered_md(config, &output_path, &expanded, dry_run, 20)
}

// =============================================================================
// ADR Rendering
// =============================================================================

/// Render an ADR to Markdown
///
/// # Errors
/// Returns an error if signature computation fails.
pub fn render_adr(adr: &AdrEntry) -> anyhow::Result<String> {
    let meta = adr.meta();
    let content = &adr.spec.content;
    let mut out = String::new();

    // Compute signature (per ADR-0003)
    let signature = compute_adr_signature(adr)?;

    // Signature header
    out.push_str(&format_signature_header(&meta.id, &signature));
    let _ = writeln!(out);

    // Title
    let _ = writeln!(out, "# {}: {}", meta.id, meta.title);
    let _ = writeln!(out);

    // Status and date
    let _ = writeln!(
        out,
        "> **Status:** {} | **Date:** {}",
        meta.status.as_ref(),
        meta.date
    );
    if let Some(ref by) = meta.superseded_by {
        let _ = writeln!(out, "> **Superseded by:** {by}");
    }
    let _ = writeln!(out);

    // References (expanded to markdown links)
    if !meta.refs.is_empty() {
        let _ = writeln!(out, "**References:** {}", render_refs(&meta.refs));
        let _ = writeln!(out);
    }

    // Context
    let _ = writeln!(out, "## Context");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", content.context);
    let _ = writeln!(out);

    // Decision
    let _ = writeln!(out, "## Decision");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", content.decision);
    let _ = writeln!(out);

    // Consequences
    let _ = writeln!(out, "## Consequences");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", content.consequences);
    let _ = writeln!(out);

    // Alternatives Considered (extended per ADR-0027)
    if !content.alternatives.is_empty() {
        use crate::model::AlternativeStatus;
        let _ = writeln!(out, "## Alternatives Considered");
        let _ = writeln!(out);
        for alt in &content.alternatives {
            // Render as subheading with status
            let status_suffix = match alt.status {
                AlternativeStatus::Considered => "",
                AlternativeStatus::Accepted => " (accepted)",
                AlternativeStatus::Rejected => " (rejected)",
            };
            let _ = writeln!(out, "### {}{}", alt.text, status_suffix);
            let _ = writeln!(out);

            // Pros
            if !alt.pros.is_empty() {
                let _ = writeln!(out, "- **Pros:** {}", alt.pros.join(", "));
            }

            // Cons
            if !alt.cons.is_empty() {
                let _ = writeln!(out, "- **Cons:** {}", alt.cons.join(", "));
            }

            // Rejection reason
            if let Some(ref reason) = alt.rejection_reason {
                let _ = writeln!(out, "- **Rejected because:** {}", reason);
            }

            let _ = writeln!(out);
        }
    }

    Ok(out)
}

/// Write rendered ADR to file
pub fn write_adr_md(config: &Config, adr: &AdrEntry, dry_run: bool) -> anyhow::Result<()> {
    let meta = adr.meta();
    let output_path = config.adr_output().join(format!("{}.md", meta.id));

    // Render and expand inline references (per ADR-0011)
    let raw = render_adr(adr)?;
    let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);

    write_rendered_md(config, &output_path, &expanded, dry_run, 15)
}

// =============================================================================
// Work Item Rendering
// =============================================================================

/// Render a Work Item to Markdown
///
/// # Errors
/// Returns an error if signature computation fails.
pub fn render_work_item(item: &WorkItemEntry) -> anyhow::Result<String> {
    let meta = item.meta();
    let content = &item.spec.content;
    let mut out = String::new();

    // Compute signature (per ADR-0003)
    let signature = compute_work_item_signature(item)?;

    // Signature header
    out.push_str(&format_signature_header(&meta.id, &signature));
    let _ = writeln!(out);

    // Title
    let _ = writeln!(out, "# {}", meta.title);
    let _ = writeln!(out);

    // Status
    let mut status_line = format!(
        "> **ID:** {} | **Status:** {}",
        meta.id,
        meta.status.as_ref()
    );
    if let Some(ref start) = meta.started {
        status_line.push_str(&format!(" | **Started:** {start}"));
    }
    if let Some(ref done) = meta.completed {
        status_line.push_str(&format!(" | **Completed:** {done}"));
    }
    let _ = writeln!(out, "{status_line}");
    let _ = writeln!(out);

    // References (expanded to markdown links)
    if !meta.refs.is_empty() {
        let _ = writeln!(out, "**References:** {}", render_refs(&meta.refs));
        let _ = writeln!(out);
    }

    // Description
    let _ = writeln!(out, "## Description");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", content.description);
    let _ = writeln!(out);

    // Journal (per ADR-0026)
    if !content.journal.is_empty() {
        let _ = writeln!(out, "## Journal");
        let _ = writeln!(out);
        for entry in &content.journal {
            // Render heading with date and optional scope
            if let Some(ref scope) = entry.scope {
                let _ = writeln!(out, "### {} · {}", entry.date, scope);
            } else {
                let _ = writeln!(out, "### {}", entry.date);
            }
            let _ = writeln!(out);
            // Render content (multi-line markdown)
            let _ = writeln!(out, "{}", entry.content);
            let _ = writeln!(out);
        }
    }

    // Acceptance Criteria
    if !content.acceptance_criteria.is_empty() {
        use crate::model::ChecklistStatus;
        let _ = writeln!(out, "## Acceptance Criteria");
        let _ = writeln!(out);
        for ac_item in &content.acceptance_criteria {
            // Indent continuation lines to keep them within the list item
            let indented_text = indent_continuation(&ac_item.text);
            let line = match ac_item.status {
                ChecklistStatus::Pending => format!("- [ ] {}", indented_text),
                ChecklistStatus::Done => format!("- [x] {}", indented_text),
                ChecklistStatus::Cancelled => format!("- ~~{}~~", indented_text),
            };
            let _ = writeln!(out, "{line}");
        }
        let _ = writeln!(out);
    }

    // Notes
    if !content.notes.is_empty() {
        let _ = writeln!(out, "## Notes");
        let _ = writeln!(out);
        for note in &content.notes {
            let _ = writeln!(out, "- {}", note);
        }
        let _ = writeln!(out);
    }

    Ok(out)
}

/// Write rendered Work Item to file
pub fn write_work_item_md(
    config: &Config,
    item: &WorkItemEntry,
    dry_run: bool,
) -> anyhow::Result<()> {
    let meta = item.meta();
    let output_path = config.work_output().join(format!("{}.md", meta.id));

    // Render and expand inline references (per ADR-0011)
    let raw = render_work_item(item)?;
    let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);

    write_rendered_md(config, &output_path, &expanded, dry_run, 15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AdrContent, AdrMeta, AdrSpec, AdrStatus, Alternative, AlternativeStatus, JournalEntry,
        WorkItemContent, WorkItemMeta, WorkItemSpec, WorkItemStatus,
    };

    const DEFAULT_PATTERN: &str = r"\[\[(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3}))\]\]";

    #[test]
    fn test_expand_inline_refs_rfc() {
        let text = "See [[RFC-0000]] for details.";
        let result = expand_inline_refs(text, DEFAULT_PATTERN);
        assert_eq!(result, "See [RFC-0000](../rfc/RFC-0000.md) for details.");
    }

    #[test]
    fn test_expand_inline_refs_clause() {
        let text = "Per [[RFC-0000:C-WORK-DEF]], work items must...";
        let result = expand_inline_refs(text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            "Per [RFC-0000:C-WORK-DEF](../rfc/RFC-0000.md#rfc-0000c-work-def), work items must..."
        );
    }

    #[test]
    fn test_expand_inline_refs_adr() {
        let text = "This follows [[ADR-0005]] guidelines.";
        let result = expand_inline_refs(text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            "This follows [ADR-0005](../adr/ADR-0005.md) guidelines."
        );
    }

    #[test]
    fn test_expand_inline_refs_multiple() {
        let text = "See [[RFC-0000]] and [[ADR-0001]] for context.";
        let result = expand_inline_refs(text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            "See [RFC-0000](../rfc/RFC-0000.md) and [ADR-0001](../adr/ADR-0001.md) for context."
        );
    }

    #[test]
    fn test_expand_inline_refs_no_match() {
        let text = "No references here.";
        let result = expand_inline_refs(text, DEFAULT_PATTERN);
        assert_eq!(result, "No references here.");
    }

    #[test]
    fn test_expand_inline_refs_invalid_pattern() {
        let text = "[[RFC-0000]] test";
        let result = expand_inline_refs(text, "[invalid(regex");
        // Invalid pattern returns text unchanged
        assert_eq!(result, "[[RFC-0000]] test");
    }

    #[test]
    fn test_ref_link_from_root_rfc() {
        let result = ref_link_from_root("RFC-0000", "docs");
        assert_eq!(result, "[RFC-0000](docs/rfc/RFC-0000.md)");
    }

    #[test]
    fn test_ref_link_from_root_clause() {
        let result = ref_link_from_root("RFC-0000:C-WORK-DEF", "docs");
        assert_eq!(
            result,
            "[RFC-0000:C-WORK-DEF](docs/rfc/RFC-0000.md#rfc-0000c-work-def)"
        );
    }

    #[test]
    fn test_ref_link_from_root_adr() {
        let result = ref_link_from_root("ADR-0005", "docs");
        assert_eq!(result, "[ADR-0005](docs/adr/ADR-0005.md)");
    }

    #[test]
    fn test_ref_link_from_root_custom_path() {
        let result = ref_link_from_root("RFC-0001", "documentation");
        assert_eq!(result, "[RFC-0001](documentation/rfc/RFC-0001.md)");
    }

    #[test]
    fn test_expand_inline_refs_from_root() {
        let text = "Per [[RFC-0002:C-RESOURCE-MODEL]], resources use verb pattern.";
        let result = expand_inline_refs_from_root(text, DEFAULT_PATTERN, "docs");
        assert_eq!(
            result,
            "Per [RFC-0002:C-RESOURCE-MODEL](docs/rfc/RFC-0002.md#rfc-0002c-resource-model), resources use verb pattern."
        );
    }

    #[test]
    fn test_expand_inline_refs_from_root_multiple() {
        let text = "See [[RFC-0000]] and [[ADR-0018]] for details.";
        let result = expand_inline_refs_from_root(text, DEFAULT_PATTERN, "docs");
        assert_eq!(
            result,
            "See [RFC-0000](docs/rfc/RFC-0000.md) and [ADR-0018](docs/adr/ADR-0018.md) for details."
        );
    }

    // Work item inline reference tests (per ADR-0020 ID formats)
    // Constructs strings at runtime to avoid source_scan matching test fixtures

    fn wi_ref(id: &str) -> String {
        format!("[[{}]]", id)
    }

    #[test]
    fn test_expand_inline_refs_work_item_sequential() {
        let id = "WI-9999-01-26-001";
        let text = format!("See {} for task details.", wi_ref(id));
        let result = expand_inline_refs(&text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            format!("See [{}](../work/{}.md) for task details.", id, id)
        );
    }

    #[test]
    fn test_expand_inline_refs_work_item_author_hash() {
        let id = "WI-9999-01-26-a7f3-001";
        let text = format!("See {} for task details.", wi_ref(id));
        let result = expand_inline_refs(&text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            format!("See [{}](../work/{}.md) for task details.", id, id)
        );
    }

    #[test]
    fn test_expand_inline_refs_work_item_random() {
        let id = "WI-9999-01-26-b2c9";
        let text = format!("See {} for task details.", wi_ref(id));
        let result = expand_inline_refs(&text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            format!("See [{}](../work/{}.md) for task details.", id, id)
        );
    }

    #[test]
    fn test_expand_inline_refs_work_item_mixed() {
        let wi_id = "WI-9999-01-26-001";
        let text = format!("Per [[RFC-0000]], see {} and [[ADR-0020]].", wi_ref(wi_id));
        let result = expand_inline_refs(&text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            format!(
                "Per [RFC-0000](../rfc/RFC-0000.md), see [{}](../work/{}.md) and [ADR-0020](../adr/ADR-0020.md).",
                wi_id, wi_id
            )
        );
    }

    // Tests for render_adr with new Alternative fields per [[ADR-0027]]

    #[test]
    fn test_render_adr_alternatives_with_pros_cons() {
        let adr = AdrEntry {
            spec: AdrSpec {
                govctl: AdrMeta {
                    schema: 1,
                    id: "ADR-9999".to_string(),
                    title: "Test ADR".to_string(),
                    status: AdrStatus::Accepted,
                    date: "2026-02-22".to_string(),
                    superseded_by: None,
                    refs: vec![],
                },
                content: AdrContent {
                    context: "Test context".to_string(),
                    decision: "Test decision".to_string(),
                    consequences: "Test consequences".to_string(),
                    alternatives: vec![Alternative {
                        text: "Option A".to_string(),
                        status: AlternativeStatus::Considered,
                        pros: vec!["Fast".to_string(), "Cheap".to_string()],
                        cons: vec!["Less reliable".to_string()],
                        rejection_reason: None,
                    }],
                },
            },
            path: std::path::PathBuf::new(),
        };

        let result = render_adr(&adr).unwrap();
        assert!(result.contains("### Option A"));
        assert!(result.contains("- **Pros:** Fast, Cheap"));
        assert!(result.contains("- **Cons:** Less reliable"));
    }

    #[test]
    fn test_render_adr_alternatives_rejected_with_reason() {
        let adr = AdrEntry {
            spec: AdrSpec {
                govctl: AdrMeta {
                    schema: 1,
                    id: "ADR-9998".to_string(),
                    title: "Test ADR Rejected".to_string(),
                    status: AdrStatus::Accepted,
                    date: "2026-02-22".to_string(),
                    superseded_by: None,
                    refs: vec![],
                },
                content: AdrContent {
                    context: "Test context".to_string(),
                    decision: "Test decision".to_string(),
                    consequences: "Test consequences".to_string(),
                    alternatives: vec![Alternative {
                        text: "Option B".to_string(),
                        status: AlternativeStatus::Rejected,
                        pros: vec![],
                        cons: vec!["Too expensive".to_string()],
                        rejection_reason: Some("Budget constraints".to_string()),
                    }],
                },
            },
            path: std::path::PathBuf::new(),
        };

        let result = render_adr(&adr).unwrap();
        assert!(result.contains("### Option B (rejected)"));
        assert!(result.contains("- **Rejected because:** Budget constraints"));
    }

    // Tests for render_work_item with journal field per [[ADR-0026]]

    #[test]
    fn test_render_work_item_journal() {
        let item = WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 1,
                    id: "WI-2026-02-22-001".to_string(),
                    title: "Test Work Item".to_string(),
                    status: WorkItemStatus::Active,
                    created: Some("2026-02-22".to_string()),
                    started: Some("2026-02-22".to_string()),
                    completed: None,
                    refs: vec![],
                },
                content: WorkItemContent {
                    description: "Test description".to_string(),
                    journal: vec![JournalEntry {
                        date: "2026-02-22".to_string(),
                        scope: None,
                        content: "Started implementation".to_string(),
                    }],
                    acceptance_criteria: vec![],
                    notes: vec![],
                },
            },
            path: std::path::PathBuf::new(),
        };

        let result = render_work_item(&item).unwrap();
        assert!(result.contains("## Journal"));
        assert!(result.contains("### 2026-02-22"));
        assert!(result.contains("Started implementation"));
    }

    #[test]
    fn test_render_work_item_journal_with_scope() {
        let item = WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 1,
                    id: "WI-2026-02-22-002".to_string(),
                    title: "Test Work Item with Scope".to_string(),
                    status: WorkItemStatus::Active,
                    created: Some("2026-02-22".to_string()),
                    started: Some("2026-02-22".to_string()),
                    completed: None,
                    refs: vec![],
                },
                content: WorkItemContent {
                    description: "Test description".to_string(),
                    journal: vec![
                        JournalEntry {
                            date: "2026-02-22".to_string(),
                            scope: Some("API".to_string()),
                            content: "Created endpoint".to_string(),
                        },
                        JournalEntry {
                            date: "2026-02-23".to_string(),
                            scope: Some("Testing".to_string()),
                            content: "Added unit tests".to_string(),
                        },
                    ],
                    acceptance_criteria: vec![],
                    notes: vec![],
                },
            },
            path: std::path::PathBuf::new(),
        };

        let result = render_work_item(&item).unwrap();
        assert!(result.contains("### 2026-02-22 · API"));
        assert!(result.contains("Created endpoint"));
        assert!(result.contains("### 2026-02-23 · Testing"));
        assert!(result.contains("Added unit tests"));
    }
}
