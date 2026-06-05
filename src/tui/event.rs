//! Event handling for TUI.

use super::app::{App, View};
use super::ui;
use crate::diagnostic::DiagnosticResult;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use std::io::Stdout;
use std::time::Duration;

/// Run the main event loop
pub fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> DiagnosticResult<()> {
    loop {
        terminal
            .draw(|frame| ui::draw(frame, app))
            .map_err(|err| super::terminal_error("draw TUI frame", err))?;

        if event::poll(Duration::from_millis(100))
            .map_err(|err| super::terminal_error("poll terminal event", err))?
            && let Event::Key(key) =
                event::read().map_err(|err| super::terminal_error("read terminal event", err))?
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
                View::RfcList
                | View::ClauseList
                | View::AdrList
                | View::WorkList
                | View::GuardList
                | View::ReleaseList
                | View::TagList
                | View::LoopList
                | View::DiagnosticList => {
                    if app.filter_mode {
                        handle_filter_input(app, key);
                    } else {
                        handle_list_keys(app, key);
                    }
                }
                View::Search => {
                    if app.search_mode {
                        handle_search_input(app, key);
                    } else if app.filter_mode {
                        handle_filter_input(app, key);
                    } else {
                        handle_search_keys(app, key);
                    }
                }
                View::LoopDetail(_) => handle_loop_detail_keys(app, key),
                View::RfcDetail(_) => handle_rfc_detail_keys(app, key),
                View::AdrDetail(_)
                | View::WorkDetail(_)
                | View::GuardDetail(_)
                | View::ClauseDetail(_, _) => handle_detail_keys(app, key),
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
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: dashboard entry point.
        KeyCode::Char('1') | KeyCode::Char('r') => app.go_to(View::RfcList),
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: dashboard entry point.
        KeyCode::Char('2') | KeyCode::Char('c') => app.go_to(View::ClauseList),
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: dashboard entry point.
        KeyCode::Char('3') | KeyCode::Char('a') => app.go_to(View::AdrList),
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: dashboard entry point.
        KeyCode::Char('4') | KeyCode::Char('w') => app.go_to(View::WorkList),
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: dashboard entry point.
        KeyCode::Char('5') | KeyCode::Char('g') => app.go_to(View::GuardList),
        // Implements [[RFC-0007:C-SEARCH]]: enter search with single-focus input.
        KeyCode::Char('6') | KeyCode::Char('s') => {
            app.go_to(View::Search);
            app.enter_search_mode();
        }
        // Implements [[RFC-0007:C-LOOP-VIEWS]]: dashboard entry point.
        KeyCode::Char('7') | KeyCode::Char('l') => app.go_to(View::LoopList),
        // Implements [[RFC-0007:C-DIAGNOSTICS]]: dashboard entry point.
        KeyCode::Char('8') | KeyCode::Char('d') => app.go_to(View::DiagnosticList),
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: dashboard entry point.
        KeyCode::Char('9') => app.go_to(View::ReleaseList),
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: dashboard entry point.
        KeyCode::Char('t') => app.go_to(View::TagList),
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

fn handle_search_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('g') => app.select_top(),
        KeyCode::Char('G') => app.select_bottom(),
        // Implements [[RFC-0007:C-SEARCH]]: selected result opens contextual detail.
        KeyCode::Enter => app.enter_detail(),
        // Implements [[RFC-0007:C-SEARCH]]: search keeps a single focused input mode.
        KeyCode::Char('/') | KeyCode::Char('e') => app.enter_search_mode(),
        // Implements [[RFC-0007:C-HUMAN-UX]]: keyboard-only return navigation.
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
}

fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        // Implements [[RFC-0007:C-HUMAN-UX]]: leave focused input without mutating state.
        KeyCode::Esc => app.exit_search_mode(),
        // Implements [[RFC-0007:C-SEARCH]]: submit query through the CLI search contract.
        KeyCode::Enter => {
            app.exit_search_mode();
            app.submit_search();
        }
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Char(ch) => app.push_search_char(ch),
        _ => {}
    }
}

fn handle_rfc_detail_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.clause_next(),
        KeyCode::Char('k') | KeyCode::Up => app.clause_prev(),
        // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: browse from RFC to clause detail.
        KeyCode::Enter => app.enter_clause_detail(),
        // Implements [[RFC-0007:C-HUMAN-UX]]: keyboard-only return navigation.
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
        // Implements [[RFC-0007:C-HUMAN-UX]]: keyboard-only return navigation.
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
}

fn handle_loop_detail_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        // Implements [[RFC-0007:C-LOOP-DAG]]: selection changes preserve DAG context.
        KeyCode::Char('j') | KeyCode::Down => app.loop_item_next(),
        // Implements [[RFC-0007:C-LOOP-DAG]]: selection changes preserve DAG context.
        KeyCode::Char('k') | KeyCode::Up => app.loop_item_prev(),
        // Implements [[RFC-0007:C-HUMAN-UX]]: keyboard-only return navigation.
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
