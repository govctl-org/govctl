use crate::config::Config;
use crate::ui;
use std::io::Write;

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
) -> anyhow::Result<()> {
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
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(output_path)?;
        file.write_all(content.as_bytes())?;
        ui::rendered(&display_path);
    }

    Ok(())
}
