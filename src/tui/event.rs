//! Event handling for TUI.

use super::app::{App, View};
use super::ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
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
                View::Dashboard => handle_dashboard_keys(app, key),
                View::RfcList | View::AdrList | View::WorkList => {
                    if app.filter_mode {
                        handle_filter_input(app, key);
                    } else {
                        handle_list_keys(app, key);
                    }
                }
                View::RfcDetail(_) => handle_rfc_detail_keys(app, key),
                View::AdrDetail(_) | View::WorkDetail(_) | View::ClauseDetail(_, _) => {
                    handle_detail_keys(app, key)
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn is_ctrl(key: &KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
}

fn handle_dashboard_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('1') | KeyCode::Char('r') => app.go_to(View::RfcList),
        KeyCode::Char('2') | KeyCode::Char('a') => app.go_to(View::AdrList),
        KeyCode::Char('3') | KeyCode::Char('w') => app.go_to(View::WorkList),
        KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
}

fn handle_list_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('g') => app.select_top(),
        KeyCode::Char('G') => app.select_bottom(),
        KeyCode::Char('d') if is_ctrl(&key) => app.select_half_page_down(),
        KeyCode::Char('u') if is_ctrl(&key) => app.select_half_page_up(),
        KeyCode::PageDown => app.select_half_page_down(),
        KeyCode::PageUp => app.select_half_page_up(),
        KeyCode::Char('n') if app.filter_active() => app.select_next(),
        KeyCode::Char('p') if app.filter_active() => app.select_prev(),
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

fn handle_rfc_detail_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.clause_next(),
        KeyCode::Char('k') | KeyCode::Up => app.clause_prev(),
        KeyCode::Enter => app.enter_clause_detail(),
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
}

fn handle_detail_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
        KeyCode::Char('d') if is_ctrl(&key) => app.scroll_half_page_down(),
        KeyCode::Char('u') if is_ctrl(&key) => app.scroll_half_page_up(),
        KeyCode::PageDown => app.scroll_page_down(),
        KeyCode::PageUp => app.scroll_page_up(),
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
}

fn handle_filter_input(app: &mut App, key: KeyEvent) {
    match key.code {
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
