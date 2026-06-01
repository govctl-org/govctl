use owo_colors::OwoColorize;
use std::path::Path;

/// Check if stderr supports colors (TTY detection + NO_COLOR)
///
/// Implements [[ADR-0017]] terminal capability detection:
/// - Auto-detect TTY
/// - Respect `NO_COLOR` environment variable
pub(super) fn use_colors() -> bool {
    // Respect NO_COLOR environment variable per https://no-color.org/
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    supports_color::on(supports_color::Stream::Stderr).is_some()
}

/// Check if stdout supports colors (TTY detection + NO_COLOR)
///
/// Use this for commands that output to stdout (e.g., `list`, `status`).
pub fn stdout_supports_color() -> bool {
    // Respect NO_COLOR environment variable per https://no-color.org/
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    supports_color::on(supports_color::Stream::Stdout).is_some()
}

/// Format a file path (cyan)
pub fn path_str(p: &Path) -> String {
    if use_colors() {
        format!("{}", p.display().cyan())
    } else {
        format!("{}", p.display())
    }
}
