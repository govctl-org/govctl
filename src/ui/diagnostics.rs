use super::color::use_colors;
use crate::diagnostic::{Diagnostic, DiagnosticLevel};
use owo_colors::OwoColorize;

/// Format a diagnostic message
pub fn diagnostic(diag: &Diagnostic) {
    if use_colors() {
        let level_str = match diag.level {
            DiagnosticLevel::Error => "error".red().bold().to_string(),
            DiagnosticLevel::Warning => "warning".yellow().bold().to_string(),
            DiagnosticLevel::Info => "info".cyan().bold().to_string(),
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
            DiagnosticLevel::Info => "info",
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
