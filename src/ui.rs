//! CLI output formatting with colors (per ADR-0005).
//!
//! Provides consistent, colorized output for all CLI commands.
//! Colors auto-disable when output is not a TTY (agent-friendly).

use owo_colors::OwoColorize;
use std::fmt::Display;
use std::path::Path;

/// Check if stderr supports colors (TTY detection)
fn use_colors() -> bool {
    supports_color::on(supports_color::Stream::Stderr).is_some()
}

// =============================================================================
// Color Helpers
// =============================================================================

/// Format a success message (green checkmark prefix)
pub fn success(msg: impl Display) {
    if use_colors() {
        eprintln!("{} {}", "✓".green(), msg);
    } else {
        eprintln!("✓ {}", msg);
    }
}

/// Format an info/action message (no special prefix)
pub fn info(msg: impl Display) {
    eprintln!("{}", msg);
}

/// Format a created item message
pub fn created(kind: &str, path: &Path) {
    if use_colors() {
        eprintln!("{} {}: {}", "Created".green(), kind, path.display().cyan());
    } else {
        eprintln!("Created {}: {}", kind, path.display());
    }
}

/// Format a file path (cyan)
#[allow(dead_code)]
pub fn path_str(p: &Path) -> String {
    if use_colors() {
        format!("{}", p.display().cyan())
    } else {
        format!("{}", p.display())
    }
}

/// Format an artifact ID (cyan, bold)
#[allow(dead_code)]
pub fn id_str(id: &str) -> String {
    if use_colors() {
        format!("{}", id.cyan().bold())
    } else {
        id.to_string()
    }
}

/// Format a field set message
pub fn field_set(id: &str, field: &str, value: &str) {
    if use_colors() {
        eprintln!(
            "Set {}.{} = {}",
            id.cyan().bold(),
            field.yellow(),
            value.white()
        );
    } else {
        eprintln!("Set {}.{} = {}", id, field, value);
    }
}

/// Format a field add message
pub fn field_added(id: &str, field: &str, value: &str) {
    if use_colors() {
        eprintln!(
            "Added '{}' to {}.{}",
            value.white(),
            id.cyan().bold(),
            field.yellow()
        );
    } else {
        eprintln!("Added '{}' to {}.{}", value, id, field);
    }
}

/// Format a field remove message
pub fn field_removed(id: &str, field: &str, value: &str) {
    if use_colors() {
        eprintln!(
            "Removed '{}' from {}.{}",
            value.white(),
            id.cyan().bold(),
            field.yellow()
        );
    } else {
        eprintln!("Removed '{}' from {}.{}", value, id, field);
    }
}

/// Format an item move message
pub fn moved(filename: &str, status: &str) {
    if use_colors() {
        eprintln!("Moved {} to {}", filename.cyan(), status.green().bold());
    } else {
        eprintln!("Moved {} to {}", filename, status);
    }
}

/// Format a status transition message
pub fn transitioned(id: &str, action: &str, target: &str) {
    if use_colors() {
        eprintln!(
            "{} {}: {}",
            action,
            id.cyan().bold(),
            target.green()
        );
    } else {
        eprintln!("{} {}: {}", action, id, target);
    }
}

/// Format a phase advance message
pub fn phase_advanced(id: &str, phase: &str) {
    if use_colors() {
        eprintln!(
            "Advanced {} to phase: {}",
            id.cyan().bold(),
            phase.green()
        );
    } else {
        eprintln!("Advanced {} to phase: {}", id, phase);
    }
}

/// Format a version bump message
pub fn version_bumped(id: &str, version: &str) {
    if use_colors() {
        eprintln!("Bumped {} to {}", id.cyan().bold(), version.green().bold());
    } else {
        eprintln!("Bumped {} to {}", id, version);
    }
}

/// Format a changelog change added message
pub fn changelog_change_added(id: &str, version: &str, change: &str) {
    if use_colors() {
        eprintln!(
            "Added change to {} v{}: {}",
            id.cyan().bold(),
            version.green(),
            change
        );
    } else {
        eprintln!("Added change to {} v{}: {}", id, version, change);
    }
}

/// Format a checklist item tick message
pub fn ticked(item: &str, status: &str) {
    if use_colors() {
        eprintln!("Marked '{}' as {}", item.white(), status.green());
    } else {
        eprintln!("Marked '{}' as {}", item, status);
    }
}

/// Format an accepted message
pub fn accepted(kind: &str, id: &str) {
    if use_colors() {
        eprintln!("Accepted {}: {}", kind, id.cyan().bold());
    } else {
        eprintln!("Accepted {}: {}", kind, id);
    }
}

/// Format a deprecated message
pub fn deprecated(kind: &str, id: &str) {
    if use_colors() {
        eprintln!("Deprecated {}: {}", kind, id.yellow().bold());
    } else {
        eprintln!("Deprecated {}: {}", kind, id);
    }
}

/// Format a superseded message
pub fn superseded(kind: &str, id: &str, by: &str) {
    if use_colors() {
        eprintln!("Superseded {}: {}", kind, id.yellow().bold());
        eprintln!("  Replaced by: {}", by.cyan().bold());
    } else {
        eprintln!("Superseded {}: {}", kind, id);
        eprintln!("  Replaced by: {}", by);
    }
}

/// Format a rendered file message
pub fn rendered(path: &Path) {
    if use_colors() {
        eprintln!("{}: {}", "Rendered".green(), path.display().cyan());
    } else {
        eprintln!("Rendered: {}", path.display());
    }
}

/// Format "not found" message
pub fn not_found(kind: &str, location: &Path) {
    if use_colors() {
        eprintln!("No {}s found in {}", kind, location.display().cyan());
    } else {
        eprintln!("No {}s found in {}", kind, location.display());
    }
}

/// Format check summary header
pub fn check_header() {
    if use_colors() {
        eprintln!("{}:", "Checked".bold());
    } else {
        eprintln!("Checked:");
    }
}

/// Format check count line
pub fn check_count(count: usize, kind: &str) {
    if use_colors() {
        eprintln!("  {} {}", count.to_string().cyan().bold(), kind);
    } else {
        eprintln!("  {} {}", count, kind);
    }
}

/// Format render summary
pub fn render_summary(count: usize, kind: &str) {
    if use_colors() {
        eprintln!(
            "{} Rendered {} {}(s)",
            "✓".green(),
            count.to_string().cyan().bold(),
            kind
        );
    } else {
        eprintln!("✓ Rendered {} {}(s)", count, kind);
    }
}

// =============================================================================
// Diagnostic Formatting
// =============================================================================

use crate::diagnostic::{Diagnostic, DiagnosticLevel};

/// Format a diagnostic message
pub fn diagnostic(diag: &Diagnostic) {
    if use_colors() {
        let level_str = match diag.level {
            DiagnosticLevel::Error => "error".red().bold().to_string(),
            DiagnosticLevel::Warning => "warning".yellow().bold().to_string(),
        };
        eprintln!(
            "{}[{}]: {} ({})",
            level_str,
            diag.code.code().bright_black(),
            diag.message,
            diag.file.cyan()
        );
    } else {
        let level_str = match diag.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
        };
        eprintln!(
            "{}[{}]: {} ({})",
            level_str,
            diag.code.code(),
            diag.message,
            diag.file
        );
    }
}
