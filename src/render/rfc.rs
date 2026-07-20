use super::{RenderProjection, render_refs, write_expanded_rendered_md};
use crate::config::Config;
use crate::diagnostic::DiagnosticResult;
use crate::model::{ClauseEntry, ClauseKind, ClauseStatus, RfcIndex, RfcStatus};
use crate::signature::{compute_rfc_signature, format_signature_header};
use std::fmt::Write as FmtWrite;

/// Render an RFC to Markdown
///
/// # Errors
/// Returns an error if signature computation fails.
pub fn render_rfc(rfc: &RfcIndex) -> DiagnosticResult<String> {
    render_rfc_with_projection(rfc, RenderProjection::Archive, None)
}

/// Render an RFC using the selected lifecycle projection.
///
/// # Errors
/// Returns an error if signature computation fails.
pub fn render_rfc_with_projection(
    rfc: &RfcIndex,
    projection: RenderProjection,
    superseded_by: Option<&str>,
) -> DiagnosticResult<String> {
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

    let suppress_body =
        projection == RenderProjection::Current && rfc.rfc.status == RfcStatus::Deprecated;

    if !rfc.rfc.owners.is_empty() {
        let _ = writeln!(out, "> **Owners:** {}", rfc.rfc.owners.join(", "));
    }
    if !rfc.rfc.tags.is_empty() {
        let _ = writeln!(out, "> **Tags:** `{}`", rfc.rfc.tags.join("`, `"));
    }
    if let Some(ref supersedes) = rfc.rfc.supersedes {
        let _ = writeln!(out, "> **Supersedes:** {supersedes}");
    }
    if suppress_body && let Some(superseded_by) = superseded_by {
        let _ = writeln!(out, "> **Superseded by:** {superseded_by}");
    }
    let _ = writeln!(out);

    // References (expanded to markdown links)
    if !rfc.rfc.refs.is_empty() {
        let _ = writeln!(out, "**References:** {}", render_refs(&rfc.rfc.refs));
        let _ = writeln!(out);
    }

    // [[RFC-0002:C-SHOW-PROJECTION]]: a deprecated RFC is a metadata-only
    // tombstone in the current projection; archival rendering remains complete.
    if suppress_body {
        return Ok(out);
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
                render_clause_with_projection(&mut out, &rfc.rfc.rfc_id, clause, projection);
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
pub fn render_clause(out: &mut String, rfc_id: &str, clause: &ClauseEntry) {
    render_clause_with_projection(out, rfc_id, clause, RenderProjection::Archive);
}

/// Render a single Clause using the selected lifecycle projection.
pub fn render_clause_with_projection(
    out: &mut String,
    rfc_id: &str,
    clause: &ClauseEntry,
    projection: RenderProjection,
) {
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

    // Keep lifecycle state visible before historical text per [[RFC-0001:C-CLAUSE-STATUS]].
    match spec.status {
        ClauseStatus::Active => {}
        ClauseStatus::Deprecated => {
            let _ = writeln!(out, "> **Status:** deprecated");
        }
        ClauseStatus::Superseded => {
            let _ = writeln!(out, "> **Status:** superseded");
        }
    }

    if let Some(ref by) = spec.superseded_by {
        let _ = writeln!(out, "> **Superseded by:** {by}");
    }

    if spec.status != ClauseStatus::Active || spec.superseded_by.is_some() {
        let _ = writeln!(out);
    }

    let suppress_body = projection == RenderProjection::Current
        && matches!(
            spec.status,
            ClauseStatus::Deprecated | ClauseStatus::Superseded
        );
    if !suppress_body {
        let _ = writeln!(out, "{}", spec.text);
        let _ = writeln!(out);
    }

    // Tags
    if !spec.tags.is_empty() {
        let _ = writeln!(out, "> **Tags:** `{}`", spec.tags.join("`, `"));
        let _ = writeln!(out);
    }

    // Since version
    if let Some(ref since) = spec.since {
        let _ = writeln!(out, "*Since: v{since}*");
        let _ = writeln!(out);
    }
}

/// Write rendered RFC to file
pub fn write_rfc(config: &Config, rfc: &RfcIndex, dry_run: bool) -> DiagnosticResult<()> {
    let output_path = config.rfc_output().join(format!("{}.md", rfc.rfc.rfc_id));

    let raw = render_rfc(rfc)?;
    write_expanded_rendered_md(config, &output_path, &raw, dry_run, 20)
}
