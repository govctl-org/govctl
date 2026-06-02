use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult};
use crate::ui;
use std::io::Write;
use std::path::Path;

use super::expand_inline_refs;

/// Write rendered markdown to file with common formatting.
///
/// Handles dry-run preview, directory creation, and consistent formatting.
/// `preview_lines` controls how many lines to show in dry-run mode.
pub(super) fn write_rendered_md(
    config: &Config,
    output_path: &std::path::Path,
    content: &str,
    dry_run: bool,
    preview_lines: usize,
) -> DiagnosticResult<()> {
    let content = format!("{}\n", content.trim_end());
    let display_path = config.display_path(output_path);

    if dry_run {
        ui::dry_run_preview(&display_path);
        for line in content.lines().take(preview_lines) {
            ui::preview_line(line);
        }
        ui::preview_truncated();
    } else {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                Diagnostic::io_error(
                    "create render output directory",
                    err,
                    config.display_path(parent).display().to_string(),
                )
            })?;
        }
        let mut file = std::fs::File::create(output_path).map_err(|err| {
            Diagnostic::io_error(
                "create render output file",
                err,
                display_path.display().to_string(),
            )
        })?;
        file.write_all(content.as_bytes()).map_err(|err| {
            Diagnostic::io_error(
                "write render output file",
                err,
                display_path.display().to_string(),
            )
        })?;
        ui::rendered(&display_path);
    }

    Ok(())
}

pub(super) fn write_expanded_rendered_md(
    config: &Config,
    output_path: &Path,
    raw_markdown: &str,
    dry_run: bool,
    preview_lines: usize,
) -> DiagnosticResult<()> {
    let expanded = expand_inline_refs(raw_markdown, &config.source_scan.pattern);
    write_rendered_md(config, output_path, &expanded, dry_run, preview_lines)
}
