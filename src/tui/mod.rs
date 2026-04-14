//! TUI module for interactive dashboard.
//!
//! This module provides an interactive terminal UI for browsing
//! RFCs, ADRs, and Work Items.

mod app;
mod event;
mod ui;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLevel};
use crate::load::load_project;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

pub use app::App;

fn project_load_error(diags: Vec<Diagnostic>, gov_root: &std::path::Path) -> anyhow::Error {
    let first = diags.first().cloned();
    diags
        .into_iter()
        .find(|d| d.level == DiagnosticLevel::Error)
        .or(first)
        .map(anyhow::Error::from)
        .unwrap_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                "Failed to load project",
                gov_root.display().to_string(),
            )
            .into()
        })
}

/// Run the TUI application
pub fn run(config: &Config) -> Result<()> {
    // Load project data
    let index =
        load_project(config).map_err(|diags| project_load_error(diags, &config.gov_root))?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(index);

    // Run event loop
    let result = event::run_event_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_load_error_prefers_error_diagnostic() -> Result<(), Box<dyn std::error::Error>>
    {
        let err = project_load_error(
            vec![
                Diagnostic::new(DiagnosticCode::W0109WorkNoActive, "warning", "warn"),
                Diagnostic::new(DiagnosticCode::E0302AdrNotFound, "missing adr", "gov/adr"),
            ],
            std::path::Path::new("gov"),
        );

        let diag = err
            .downcast_ref::<Diagnostic>()
            .ok_or("expected Diagnostic")?;
        assert_eq!(diag.code, DiagnosticCode::E0302AdrNotFound);
        assert_eq!(diag.message, "missing adr");
        Ok(())
    }

    #[test]
    fn test_project_load_error_uses_warning_when_no_errors_exist()
    -> Result<(), Box<dyn std::error::Error>> {
        let err = project_load_error(
            vec![Diagnostic::new(
                DiagnosticCode::W0109WorkNoActive,
                "warning only",
                "gov/work",
            )],
            std::path::Path::new("gov"),
        );

        let diag = err
            .downcast_ref::<Diagnostic>()
            .ok_or("expected Diagnostic")?;
        assert_eq!(diag.code, DiagnosticCode::W0109WorkNoActive);
        assert_eq!(diag.message, "warning only");
        Ok(())
    }

    #[test]
    fn test_project_load_error_falls_back_when_empty() -> Result<(), Box<dyn std::error::Error>> {
        let err = project_load_error(vec![], std::path::Path::new("gov"));

        let diag = err
            .downcast_ref::<Diagnostic>()
            .ok_or("expected Diagnostic")?;
        assert_eq!(diag.code, DiagnosticCode::E0501ConfigInvalid);
        assert_eq!(diag.message, "Failed to load project");
        assert_eq!(diag.file, "gov");
        Ok(())
    }
}
