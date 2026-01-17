//! SSOT to Markdown rendering.

use crate::config::Config;
use crate::model::{AdrEntry, ClauseKind, ClauseStatus, RfcIndex, WorkItemEntry};
use std::fmt::Write as FmtWrite;
use std::io::Write;

/// Render an RFC to Markdown
pub fn render_rfc(rfc: &RfcIndex) -> String {
    let mut out = String::new();

    // YAML frontmatter (for compatibility with existing tooling)
    writeln!(out, "---").unwrap();
    writeln!(out, "govctl:").unwrap();
    writeln!(out, "  schema: 1").unwrap();
    writeln!(out, "  id: {}", rfc.rfc.rfc_id).unwrap();
    writeln!(out, "  title: \"{}\"", rfc.rfc.title).unwrap();
    writeln!(out, "  kind: rfc").unwrap();
    writeln!(out, "  status: {}", rfc.rfc.status.as_ref()).unwrap();
    writeln!(out, "  phase: {}", rfc.rfc.phase.as_ref()).unwrap();
    writeln!(out, "  owners: {:?}", rfc.rfc.owners).unwrap();
    writeln!(out, "  created: {}", rfc.rfc.created).unwrap();
    if let Some(ref updated) = rfc.rfc.updated {
        writeln!(out, "  updated: {updated}").unwrap();
    }
    writeln!(out, "---").unwrap();
    writeln!(out).unwrap();

    // Title
    writeln!(out, "# {}: {}", rfc.rfc.rfc_id, rfc.rfc.title).unwrap();
    writeln!(out).unwrap();

    // Version info
    writeln!(
        out,
        "> **Version:** {} | **Status:** {} | **Phase:** {}",
        rfc.rfc.version,
        rfc.rfc.status.as_ref(),
        rfc.rfc.phase.as_ref()
    )
    .unwrap();
    writeln!(out).unwrap();

    // Render sections with clauses
    for (i, section) in rfc.rfc.sections.iter().enumerate() {
        writeln!(out, "---").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "## {}. {}", i + 1, section.title).unwrap();
        writeln!(out).unwrap();

        // Find and render clauses for this section
        for clause_path in &section.clauses {
            if let Some(clause) = rfc.clauses.iter().find(|c| {
                c.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| clause_path.ends_with(n))
                    .unwrap_or(false)
            }) {
                render_clause(&mut out, &rfc.rfc.rfc_id, clause);
            }
        }
    }

    // Changelog
    if !rfc.rfc.changelog.is_empty() {
        writeln!(out, "---").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "## Changelog").unwrap();
        writeln!(out).unwrap();

        for entry in &rfc.rfc.changelog {
            writeln!(out, "### v{} ({})", entry.version, entry.date).unwrap();
            writeln!(out).unwrap();
            writeln!(out, "{}", entry.summary).unwrap();
            writeln!(out).unwrap();

            if !entry.changes.is_empty() {
                for change in &entry.changes {
                    writeln!(out, "- {change}").unwrap();
                }
                writeln!(out).unwrap();
            }
        }
    }

    out
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

    writeln!(
        out,
        "### [{rfc_id}:{}] {} {kind_marker}{status_marker}",
        spec.clause_id, spec.title
    )
    .unwrap();
    writeln!(out).unwrap();

    // Clause text
    writeln!(out, "{}", spec.text).unwrap();
    writeln!(out).unwrap();

    // Superseded by notice
    if let Some(ref by) = spec.superseded_by {
        writeln!(out, "> **Superseded by:** {by}").unwrap();
        writeln!(out).unwrap();
    }

    // Since version
    if let Some(ref since) = spec.since {
        writeln!(out, "*Since: v{since}*").unwrap();
        writeln!(out).unwrap();
    }
}

/// Write rendered RFC to file
pub fn write_rfc(config: &Config, rfc: &RfcIndex, dry_run: bool) -> anyhow::Result<()> {
    let output_path = config.rfc_output().join(format!("{}.md", rfc.rfc.rfc_id));

    let content = render_rfc(rfc);

    if dry_run {
        eprintln!("Would write: {}", output_path.display());
        eprintln!("--- Content preview ---");
        // Print first 20 lines
        for line in content.lines().take(20) {
            eprintln!("{line}");
        }
        eprintln!("...");
    } else {
        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(&output_path)?;
        file.write_all(content.as_bytes())?;
        eprintln!("Rendered: {}", output_path.display());
    }

    Ok(())
}

// =============================================================================
// ADR Rendering
// =============================================================================

/// Render an ADR to Markdown
pub fn render_adr(adr: &AdrEntry) -> String {
    let meta = adr.meta();
    let content = &adr.spec.content;
    let mut out = String::new();

    // Title
    writeln!(out, "# {}: {}", meta.id, meta.title).unwrap();
    writeln!(out).unwrap();

    // Status and date
    writeln!(
        out,
        "> **Status:** {} | **Date:** {}",
        meta.status.as_ref(),
        meta.date
    )
    .unwrap();
    if let Some(ref by) = meta.superseded_by {
        writeln!(out, "> **Superseded by:** {by}").unwrap();
    }
    writeln!(out).unwrap();

    // References
    if !meta.refs.is_empty() {
        writeln!(out, "**References:** {}", meta.refs.join(", ")).unwrap();
        writeln!(out).unwrap();
    }

    // Context
    writeln!(out, "## Context").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{}", content.context).unwrap();
    writeln!(out).unwrap();

    // Decision
    writeln!(out, "## Decision").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{}", content.decision).unwrap();
    writeln!(out).unwrap();

    // Consequences
    writeln!(out, "## Consequences").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{}", content.consequences).unwrap();
    writeln!(out).unwrap();

    // Alternatives Considered
    if !content.alternatives.is_empty() {
        use crate::model::AlternativeStatus;
        writeln!(out, "## Alternatives Considered").unwrap();
        writeln!(out).unwrap();
        for alt in &content.alternatives {
            let line = match alt.status {
                AlternativeStatus::Considered => format!("- [ ] {}", alt.text),
                AlternativeStatus::Accepted => format!("- [x] {}", alt.text),
                AlternativeStatus::Rejected => format!("- ~~{}~~", alt.text),
            };
            writeln!(out, "{line}").unwrap();
        }
        writeln!(out).unwrap();
    }

    out
}

/// Write rendered ADR to file
pub fn write_adr_md(config: &Config, adr: &AdrEntry, dry_run: bool) -> anyhow::Result<()> {
    let meta = adr.meta();
    let output_dir = config.adr_output();
    let output_path = output_dir.join(format!("{}.md", meta.id));

    let rendered = render_adr(adr);

    if dry_run {
        eprintln!("Would write: {}", output_path.display());
        eprintln!("--- Content preview ---");
        for line in rendered.lines().take(15) {
            eprintln!("{line}");
        }
        eprintln!("...");
    } else {
        std::fs::create_dir_all(&output_dir)?;
        let mut file = std::fs::File::create(&output_path)?;
        file.write_all(rendered.as_bytes())?;
        eprintln!("Rendered: {}", output_path.display());
    }

    Ok(())
}

// =============================================================================
// Work Item Rendering
// =============================================================================

/// Render a Work Item to Markdown
pub fn render_work_item(item: &WorkItemEntry) -> String {
    let meta = item.meta();
    let content = &item.spec.content;
    let mut out = String::new();

    // Title
    writeln!(out, "# {}", meta.title).unwrap();
    writeln!(out).unwrap();

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
    writeln!(out, "{status_line}").unwrap();
    writeln!(out).unwrap();

    // References
    if !meta.refs.is_empty() {
        writeln!(out, "**References:** {}", meta.refs.join(", ")).unwrap();
        writeln!(out).unwrap();
    }

    // Description
    writeln!(out, "## Description").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{}", content.description).unwrap();
    writeln!(out).unwrap();

    // Acceptance Criteria
    if !content.acceptance_criteria.is_empty() {
        use crate::model::ChecklistStatus;
        writeln!(out, "## Acceptance Criteria").unwrap();
        writeln!(out).unwrap();
        for item in &content.acceptance_criteria {
            let line = match item.status {
                ChecklistStatus::Pending => format!("- [ ] {}", item.text),
                ChecklistStatus::Done => format!("- [x] {}", item.text),
                ChecklistStatus::Cancelled => format!("- ~~{}~~", item.text),
            };
            writeln!(out, "{line}").unwrap();
        }
        writeln!(out).unwrap();
    }

    // Decisions
    if !content.decisions.is_empty() {
        use crate::model::ChecklistStatus;
        writeln!(out, "## Decisions").unwrap();
        writeln!(out).unwrap();
        for item in &content.decisions {
            let line = match item.status {
                ChecklistStatus::Pending => format!("- [ ] {}", item.text),
                ChecklistStatus::Done => format!("- [x] {}", item.text),
                ChecklistStatus::Cancelled => format!("- ~~{}~~", item.text),
            };
            writeln!(out, "{line}").unwrap();
        }
        writeln!(out).unwrap();
    }

    // Notes
    if !content.notes.is_empty() {
        writeln!(out, "## Notes").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "{}", content.notes).unwrap();
        writeln!(out).unwrap();
    }

    out
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

    let rendered = render_work_item(item);

    if dry_run {
        eprintln!("Would write: {}", output_path.display());
        eprintln!("--- Content preview ---");
        for line in rendered.lines().take(15) {
            eprintln!("{line}");
        }
        eprintln!("...");
    } else {
        std::fs::create_dir_all(&output_dir)?;
        let mut file = std::fs::File::create(&output_path)?;
        file.write_all(rendered.as_bytes())?;
        eprintln!("Rendered: {}", output_path.display());
    }

    Ok(())
}
