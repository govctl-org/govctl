//! Event handling for TUI.

use super::app::{App, View};
use super::ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use std::io::Stdout;
use std::time::Duration;

/// Run the main event loop
pub fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Handle events with timeout for responsive UI
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events (not release)
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.view {
                    View::Dashboard => handle_dashboard_keys(app, key.code),
                    View::RfcList | View::AdrList | View::WorkList => {
                        handle_list_keys(app, key.code)
                    }
                    View::RfcDetail(_) | View::AdrDetail(_) | View::WorkDetail(_) => {
                        handle_detail_keys(app, key.code)
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_dashboard_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('1') | KeyCode::Char('r') => app.go_to(View::RfcList),
        KeyCode::Char('2') | KeyCode::Char('a') => app.go_to(View::AdrList),
        KeyCode::Char('3') | KeyCode::Char('w') => app.go_to(View::WorkList),
        KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
}

fn handle_list_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Enter => app.enter_detail(),
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
}

fn handle_detail_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
}
