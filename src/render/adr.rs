use super::{expand_inline_refs, render_refs, write_rendered_md};
use crate::config::Config;
use crate::model::{AdrEntry, AlternativeStatus};
use crate::signature::{compute_adr_signature, format_signature_header};
use std::fmt::Write as FmtWrite;

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
