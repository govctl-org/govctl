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

                if matches!(key.code, KeyCode::Char('?')) {
                    app.show_help = !app.show_help;
                    continue;
                }

                if app.show_help {
                    if matches!(key.code, KeyCode::Esc) {
                        app.show_help = false;
                    }
                    continue;
                }

                match app.view {
                    View::Dashboard => handle_dashboard_keys(app, key.code),
                    View::RfcList | View::AdrList | View::WorkList => {
                        if app.filter_mode {
                            handle_filter_input(app, key.code);
                        } else {
                            handle_list_keys(app, key.code);
                        }
                    }
                    View::RfcDetail(_) => handle_rfc_detail_keys(app, key.code),
                    View::AdrDetail(_) | View::WorkDetail(_) | View::ClauseDetail(_, _) => {
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
        KeyCode::Char('g') => app.select_top(),
        KeyCode::Char('G') => app.select_bottom(),
        KeyCode::Char('n') => {
            if app.filter_active() {
                app.select_next();
            }
        }
        KeyCode::Char('p') => {
            if app.filter_active() {
                app.select_prev();
            }
        }
        KeyCode::Enter => app.enter_detail(),
        KeyCode::Esc => app.go_back(),
        // Implements [[RFC-0003:C-FILTER]]
        KeyCode::Char('/') => {
            app.clear_filter();
            app.enter_filter_mode();
        }
        _ => {}
    }
}

fn handle_rfc_detail_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.clause_next(),
        KeyCode::Char('k') | KeyCode::Up => app.clause_prev(),
        KeyCode::Enter => app.enter_clause_detail(),
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

fn handle_filter_input(app: &mut App, code: KeyCode) {
    match code {
        // Implements [[RFC-0003:C-FILTER]]
        KeyCode::Esc => {
            app.clear_filter();
            app.exit_filter_mode();
        }
        KeyCode::Enter => app.exit_filter_mode(),
        KeyCode::Backspace => app.pop_filter_char(),
        KeyCode::Char(ch) => app.push_filter_char(ch),
        _ => {}
    }
}
