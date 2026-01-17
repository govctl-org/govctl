//! RFC JSON to Markdown rendering.

use crate::config::Config;
use crate::model::{ClauseKind, ClauseStatus, RfcIndex};
use std::fmt::Write as FmtWrite;
use std::io::Write;

/// Render an RFC to Markdown
pub fn render_rfc(rfc: &RfcIndex) -> String {
    let mut out = String::new();

    // YAML frontmatter (for compatibility with existing tooling)
    writeln!(out, "---").unwrap();
    writeln!(out, "phaseos:").unwrap();
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
    writeln!(out, "> **Version:** {} | **Status:** {} | **Phase:** {}",
        rfc.rfc.version,
        rfc.rfc.status.as_ref(),
        rfc.rfc.phase.as_ref()
    ).unwrap();
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
    let output_path = config
        .paths
        .rfc_output
        .join(format!("{}.md", rfc.rfc.rfc_id));

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
