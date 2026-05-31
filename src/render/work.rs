use super::{expand_inline_refs, render_refs, write_rendered_md};
use crate::config::Config;
use crate::model::{ChecklistStatus, WorkItemEntry};
use crate::signature::{compute_work_item_signature, format_signature_header};
use std::fmt::Write as FmtWrite;

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
