//! SSOT to Markdown rendering.
//!
//! Implements [[ADR-0003]] signatures and [[ADR-0011]] inline reference expansion.
//!
//! Rendered markdown files are read-only projections. Each includes:
//! - A "GENERATED" comment warning not to edit
//! - A SHA-256 signature for tampering detection
//! - Inline `[[artifact-id]]` references expanded to markdown links

use crate::config::Config;
use crate::model::{AdrEntry, WorkItemEntry};
use crate::signature::{
    compute_adr_signature, compute_work_item_signature, format_signature_header,
};
use std::fmt::Write as FmtWrite;

mod links;
mod output;
mod rfc;

pub use links::expand_inline_refs;
use links::render_refs;
use output::write_rendered_md;
pub use rfc::{render_clause, render_rfc, write_rfc};

pub fn ref_link_from_root(ref_id: &str, docs_output: &str) -> String {
    links::ref_link_from_root(ref_id, docs_output)
}

pub fn expand_inline_refs_from_root(text: &str, pattern: &str, docs_output: &str) -> String {
    links::expand_inline_refs_with_linker(text, pattern, |ref_id| {
        ref_link_from_root(ref_id, docs_output)
    })
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

    // Tags
    if !meta.tags.is_empty() {
        let _ = writeln!(out, "> **Tags:** `{}`", meta.tags.join("`, `"));
        let _ = writeln!(out);
    }

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

    // Tags
    if !meta.tags.is_empty() {
        let _ = writeln!(out, "> **Tags:** `{}`", meta.tags.join("`, `"));
        let _ = writeln!(out);
    }

    // References (expanded to markdown links)
    if !meta.refs.is_empty() {
        let _ = writeln!(out, "**References:** {}", render_refs(&meta.refs));
        let _ = writeln!(out);
    }

    // Work item dependencies (expanded to markdown links)
    if !meta.depends_on.is_empty() {
        let _ = writeln!(out, "**Depends On:** {}", render_refs(&meta.depends_on));
        let _ = writeln!(out);
    }

    // Description
    let _ = writeln!(out, "## Description");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", content.description);
    let _ = writeln!(out);

    // Legacy inline history remains renderable for existing work items per [[ADR-0047]].
    if !content.journal.is_empty() {
        let _ = writeln!(out, "## Journal");
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "> Legacy execution history preserved from older work items. Move durable takeaways to `notes` and keep new execution trace in loop state."
        );
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
            let categorized_text = format!("{}: {}", ac_item.category.as_ref(), ac_item.text);
            let indented_text = indent_continuation(&categorized_text);
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
        AdrContent, AdrMeta, AdrSpec, AdrStatus, Alternative, AlternativeStatus, ChangelogCategory,
        ChecklistItem, ChecklistStatus, JournalEntry, WorkItemContent, WorkItemMeta, WorkItemSpec,
        WorkItemStatus,
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
        let text = "See [[RFC-0000]] and [[ADR-0042]] for context.";
        let result = expand_inline_refs(text, DEFAULT_PATTERN);
        assert_eq!(
            result,
            "See [RFC-0000](../rfc/RFC-0000.md) and [ADR-0042](../adr/ADR-0042.md) for context."
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
    fn test_render_adr_alternatives_with_pros_cons() -> Result<(), Box<dyn std::error::Error>> {
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
                    tags: vec![],
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

        let result = render_adr(&adr)?;
        assert!(result.contains("### Option A"));
        assert!(result.contains("- **Pros:** Fast, Cheap"));
        assert!(result.contains("- **Cons:** Less reliable"));
        Ok(())
    }

    #[test]
    fn test_render_adr_alternatives_rejected_with_reason() -> Result<(), Box<dyn std::error::Error>>
    {
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
                    tags: vec![],
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

        let result = render_adr(&adr)?;
        assert!(result.contains("### Option B (rejected)"));
        assert!(result.contains("- **Rejected because:** Budget constraints"));
        Ok(())
    }

    // Tests for render_work_item with legacy inline history rendering per [[ADR-0047]]

    #[test]
    fn test_render_work_item_journal() -> Result<(), Box<dyn std::error::Error>> {
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
                    depends_on: vec![],
                    tags: vec![],
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
                verification: crate::model::WorkItemVerification::default(),
            },
            path: std::path::PathBuf::new(),
        };

        let result = render_work_item(&item)?;
        assert!(result.contains("## Journal"));
        assert!(result.contains("Legacy execution history"));
        assert!(result.contains("loop state"));
        assert!(result.contains("### 2026-02-22"));
        assert!(result.contains("Started implementation"));
        Ok(())
    }

    #[test]
    fn test_render_work_item_journal_with_scope() -> Result<(), Box<dyn std::error::Error>> {
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
                    depends_on: vec![],
                    tags: vec![],
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
                verification: crate::model::WorkItemVerification::default(),
            },
            path: std::path::PathBuf::new(),
        };

        let result = render_work_item(&item)?;
        assert!(result.contains("### 2026-02-22 · API"));
        assert!(result.contains("Created endpoint"));
        assert!(result.contains("### 2026-02-23 · Testing"));
        assert!(result.contains("Added unit tests"));
        Ok(())
    }

    #[test]
    fn test_render_work_item_acceptance_criteria_show_categories()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut done =
            ChecklistItem::with_category("Fix rendered category", ChangelogCategory::Fixed);
        done.status = ChecklistStatus::Done;
        let mut cancelled =
            ChecklistItem::with_category("Obsolete validation path", ChangelogCategory::Chore);
        cancelled.status = ChecklistStatus::Cancelled;

        let item = WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 1,
                    id: "WI-2026-02-22-003".to_string(),
                    title: "Test Work Item Categories".to_string(),
                    status: WorkItemStatus::Active,
                    created: Some("2026-02-22".to_string()),
                    started: Some("2026-02-22".to_string()),
                    completed: None,
                    refs: vec![],
                    depends_on: vec![],
                    tags: vec![],
                },
                content: WorkItemContent {
                    description: "Test description".to_string(),
                    journal: vec![],
                    acceptance_criteria: vec![
                        ChecklistItem::with_category(
                            "Add reviewer context",
                            ChangelogCategory::Added,
                        ),
                        done,
                        cancelled,
                    ],
                    notes: vec![],
                },
                verification: crate::model::WorkItemVerification::default(),
            },
            path: std::path::PathBuf::new(),
        };

        let result = render_work_item(&item)?;
        assert!(result.contains("- [ ] added: Add reviewer context"));
        assert!(result.contains("- [x] fixed: Fix rendered category"));
        assert!(result.contains("- ~~chore: Obsolete validation path~~"));
        Ok(())
    }
}
