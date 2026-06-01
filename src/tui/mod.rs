//! TUI module for interactive dashboard.
//!
//! This module provides an interactive terminal UI for browsing
//! RFCs, ADRs, and Work Items.

mod app;
mod event;
mod ui;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticResult};
use crate::load::load_project;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

pub use app::App;

fn terminal_error(action: &str, err: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::io_error(action, err, "tui")
}

fn project_load_error(diags: Vec<Diagnostic>, gov_root: &std::path::Path) -> Diagnostic {
    let first = diags.first().cloned();
    diags
        .into_iter()
        .find(|d| d.level == DiagnosticLevel::Error)
        .or(first)
        .unwrap_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                "Failed to load project",
                gov_root.display().to_string(),
            )
        })
}

/// Run the TUI application
pub fn run(config: &Config) -> DiagnosticResult<()> {
    // Load project data
    let index =
        load_project(config).map_err(|diags| project_load_error(diags, &config.gov_root))?;

    // Setup terminal
    enable_raw_mode().map_err(|err| terminal_error("enable raw mode", err))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|err| terminal_error("enter alternate screen", err))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|err| terminal_error("initialize terminal", err))?;

    // Create app state
    let mut app = App::new(index);

    // Run event loop
    let result = event::run_event_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().map_err(|err| terminal_error("disable raw mode", err))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|err| terminal_error("leave alternate screen", err))?;
    terminal
        .show_cursor()
        .map_err(|err| terminal_error("show cursor", err))?;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_load_error_prefers_error_diagnostic() {
        let diag = project_load_error(
            vec![
                Diagnostic::new(DiagnosticCode::W0109WorkNoActive, "warning", "warn"),
                Diagnostic::new(DiagnosticCode::E0302AdrNotFound, "missing adr", "gov/adr"),
            ],
            std::path::Path::new("gov"),
        );

        assert_eq!(diag.code, DiagnosticCode::E0302AdrNotFound);
        assert_eq!(diag.message, "missing adr");
    }

    #[test]
    fn test_project_load_error_uses_warning_when_no_errors_exist() {
        let diag = project_load_error(
            vec![Diagnostic::new(
                DiagnosticCode::W0109WorkNoActive,
                "warning only",
                "gov/work",
            )],
            std::path::Path::new("gov"),
        );

        assert_eq!(diag.code, DiagnosticCode::W0109WorkNoActive);
        assert_eq!(diag.message, "warning only");
    }

    #[test]
    fn test_project_load_error_falls_back_when_empty() {
        let diag = project_load_error(vec![], std::path::Path::new("gov"));

        assert_eq!(diag.code, DiagnosticCode::E0501ConfigInvalid);
        assert_eq!(diag.message, "Failed to load project");
        assert_eq!(diag.file, "gov");
    }
}
