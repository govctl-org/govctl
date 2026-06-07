//! TUI module for interactive dashboard.
//!
//! This module provides an interactive terminal UI for browsing
//! RFCs, ADRs, and Work Items.

mod app;
mod dag;
mod data;
mod event;
mod ui;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticResult};
use crate::load::load_project;
use crossterm::{
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
    execute!(stdout, EnterAlternateScreen)
        .map_err(|err| terminal_error("enter alternate screen", err))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|err| terminal_error("initialize terminal", err))?;

    // Create app state
    let mut app = App::with_project(config.clone(), index);

    // Run event loop
    let result = event::run_event_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().map_err(|err| terminal_error("disable raw mode", err))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|err| terminal_error("leave alternate screen", err))?;
    terminal
        .show_cursor()
        .map_err(|err| terminal_error("show cursor", err))?;

    result
}

#[cfg(test)]
mod tests;
