//! SSOT to Markdown rendering.
//!
//! Rendered markdown files are read-only projections. Each includes:
//! - A "GENERATED" comment warning not to edit
//! - A SHA-256 signature for tampering detection
//!
//! Per ADR-0003: Signatures are computed from canonicalized source content.

use crate::config::Config;
use crate::model::{AdrEntry, ClauseKind, ClauseStatus, RfcIndex, WorkItemEntry};
use crate::signature::{
    compute_adr_signature, compute_rfc_signature, compute_work_item_signature,
    format_signature_header,
};
use crate::ui;
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
    if ref_id.starts_with("RFC-") {
        if ref_id.contains(':') {
            // Clause reference: RFC-0000:C-NAME
            let rfc_id = ref_id.split(':').next().unwrap_or(ref_id);
            // Anchor: lowercase, no special chars (GitHub-style slug)
            let anchor = ref_id.to_lowercase().replace(':', "");
            format!("[{}](../rfc/{}.md#{})", ref_id, rfc_id, anchor)
        } else {
            // RFC reference
            format!("[{}](../rfc/{}.md)", ref_id, ref_id)
        }
    } else if ref_id.starts_with("ADR-") {
        format!("[{}](../adr/{}.md)", ref_id, ref_id)
    } else if ref_id.starts_with("WI-") {
        format!("[{}](../work/{}.md)", ref_id, ref_id)
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

    // YAML frontmatter (for compatibility with existing tooling)
    // Note: writeln! to String is infallible, using let _ to be explicit
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "govctl:");
    let _ = writeln!(out, "  schema: 1");
    let _ = writeln!(out, "  id: {}", rfc.rfc.rfc_id);
    let _ = writeln!(out, "  title: \"{}\"", rfc.rfc.title);
    let _ = writeln!(out, "  kind: rfc");
    let _ = writeln!(out, "  status: {}", rfc.rfc.status.as_ref());
    let _ = writeln!(out, "  phase: {}", rfc.rfc.phase.as_ref());
    let _ = writeln!(out, "  owners: {:?}", rfc.rfc.owners);
    let _ = writeln!(out, "  created: {}", rfc.rfc.created);
    if let Some(ref updated) = rfc.rfc.updated {
        let _ = writeln!(out, "  updated: {updated}");
    }
    let _ = writeln!(out, "---");
    let _ = writeln!(out);

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

    let status_marker = match spec.status {
        ClauseStatus::Active => "",
        ClauseStatus::Deprecated => " ~~DEPRECATED~~",
        ClauseStatus::Superseded => " ~~SUPERSEDED~~",
    };

    // Generate anchor for clause linking (matches ref_link anchor format)
    let anchor = clause_anchor(rfc_id, &spec.clause_id);

    let _ = writeln!(
        out,
        "### [{rfc_id}:{}] {} {kind_marker}{status_marker} <a id=\"{anchor}\"></a>",
        spec.clause_id, spec.title
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

/// Write rendered RFC to file
pub fn write_rfc(config: &Config, rfc: &RfcIndex, dry_run: bool) -> anyhow::Result<()> {
    let output_path = config.rfc_output().join(format!("{}.md", rfc.rfc.rfc_id));

    let content = render_rfc(rfc)?;

    // Trim trailing whitespace, ensure single trailing newline
    let content = format!("{}\n", content.trim_end());

    if dry_run {
        ui::dry_run_preview(&output_path);
        for line in content.lines().take(20) {
            ui::preview_line(line);
        }
        ui::preview_truncated();
    } else {
        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(&output_path)?;
        file.write_all(content.as_bytes())?;
        ui::rendered(&output_path);
    }

    Ok(())
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

    // Alternatives Considered
    if !content.alternatives.is_empty() {
        use crate::model::AlternativeStatus;
        let _ = writeln!(out, "## Alternatives Considered");
        let _ = writeln!(out);
        for alt in &content.alternatives {
            let indented_text = indent_continuation(&alt.text);
            let line = match alt.status {
                AlternativeStatus::Considered => format!("- [ ] {}", indented_text),
                AlternativeStatus::Accepted => format!("- [x] {}", indented_text),
                AlternativeStatus::Rejected => format!("- ~~{}~~", indented_text),
            };
            let _ = writeln!(out, "{line}");
        }
        let _ = writeln!(out);
    }

    Ok(out)
}

/// Write rendered ADR to file
pub fn write_adr_md(config: &Config, adr: &AdrEntry, dry_run: bool) -> anyhow::Result<()> {
    let meta = adr.meta();
    let output_dir = config.adr_output();
    let output_path = output_dir.join(format!("{}.md", meta.id));

    // Trim trailing whitespace, ensure single trailing newline
    let rendered = format!("{}\n", render_adr(adr)?.trim_end());

    if dry_run {
        ui::dry_run_preview(&output_path);
        for line in rendered.lines().take(15) {
            ui::preview_line(line);
        }
        ui::preview_truncated();
    } else {
        std::fs::create_dir_all(&output_dir)?;
        let mut file = std::fs::File::create(&output_path)?;
        file.write_all(rendered.as_bytes())?;
        ui::rendered(&output_path);
    }

    Ok(())
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
    let output_dir = config.work_output();
    let output_path = output_dir.join(format!("{}.md", meta.id));

    // Trim trailing whitespace, ensure single trailing newline
    let rendered = format!("{}\n", render_work_item(item)?.trim_end());

    if dry_run {
        ui::dry_run_preview(&output_path);
        for line in rendered.lines().take(15) {
            ui::preview_line(line);
        }
        ui::preview_truncated();
    } else {
        std::fs::create_dir_all(&output_dir)?;
        let mut file = std::fs::File::create(&output_path)?;
        file.write_all(rendered.as_bytes())?;
        ui::rendered(&output_path);
    }

    Ok(())
}
