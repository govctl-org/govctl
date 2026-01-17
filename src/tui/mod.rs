//! TUI module for interactive dashboard.
//!
//! This module provides an interactive terminal UI for browsing
//! RFCs, ADRs, and Work Items.

mod app;
mod event;
mod ui;

use crate::config::Config;
use crate::load::load_project;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

pub use app::App;

/// Run the TUI application
pub fn run(config: &Config) -> Result<()> {
    // Load project data
    let index = load_project(config).map_err(|diags| {
        anyhow::anyhow!(
            "Failed to load project: {}",
            diags
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

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
