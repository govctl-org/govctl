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
            handle_key(app, key);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

pub(super) fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if matches!(key.code, KeyCode::Char('?')) {
        app.show_help = !app.show_help;
        return;
    }

    if app.show_help {
        if matches!(key.code, KeyCode::Esc) {
            app.show_help = false;
        }
        return;
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
        | View::ClauseDetail(_, _) => {
            handle_detail_keys(app, key);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ClauseEntry, ClauseKind, ClauseSpec, ClauseStatus, ProjectIndex, RfcIndex, RfcPhase,
        RfcSpec, RfcStatus, WorkItemContent, WorkItemEntry, WorkItemMeta, WorkItemSpec,
        WorkItemStatus, WorkItemVerification,
    };
    use std::path::PathBuf;

    #[test]
    fn handle_key_routes_dashboard_and_help_overlay() {
        let mut app = App::new(ProjectIndex::default());

        handle_key(&mut app, key(KeyCode::Char('s')));
        assert_eq!(app.view, View::Search);
        assert!(app.search_mode);

        handle_key(&mut app, key(KeyCode::Char('?')));
        assert!(app.show_help);
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.show_help);
    }

    #[test]
    fn handle_key_routes_list_filter_and_selection() {
        let mut app = App::new(project_index());
        app.go_to(View::WorkList);

        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.selected, 1);
        handle_key(&mut app, key(KeyCode::Char('g')));
        assert_eq!(app.selected, 0);
        handle_key(&mut app, key(KeyCode::Char('G')));
        assert_eq!(app.selected, 1);
        handle_key(&mut app, key(KeyCode::Char('/')));
        assert!(app.filter_mode);
        handle_key(&mut app, key(KeyCode::Char('a')));
        assert_eq!(app.filter_query, "a");
        handle_key(&mut app, key(KeyCode::Backspace));
        assert!(app.filter_query.is_empty());
        handle_key(&mut app, key(KeyCode::Enter));
        assert!(!app.filter_mode);
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.view, View::WorkDetail(1));
    }

    #[test]
    fn handle_key_routes_search_modes() {
        let mut app = App::new(ProjectIndex::default());
        app.go_to(View::Search);

        handle_key(&mut app, key(KeyCode::Char('e')));
        assert!(app.search_mode);
        handle_key(&mut app, key(KeyCode::Char('r')));
        handle_key(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.search_query, "rf");
        handle_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.search_query, "r");
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.search_mode);
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn handle_key_routes_detail_loop_and_rfc_detail_keys() {
        let mut app = App::new(project_index());
        app.view = View::WorkDetail(0);
        app.content_height = 6;

        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.scroll, 1);
        handle_key(&mut app, ctrl_key(KeyCode::Char('d')));
        assert_eq!(app.scroll, 4);
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.view, View::WorkList);

        app.view = View::RfcDetail(0);
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.view, View::ClauseDetail(0, 0));
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn project_index() -> ProjectIndex {
        ProjectIndex {
            rfcs: vec![RfcIndex {
                rfc: RfcSpec {
                    rfc_id: "RFC-0001".to_string(),
                    title: "RFC".to_string(),
                    version: "0.1.0".to_string(),
                    status: RfcStatus::Normative,
                    phase: RfcPhase::Impl,
                    owners: vec![],
                    created: "2026-06-07".to_string(),
                    updated: None,
                    supersedes: None,
                    refs: vec![],
                    tags: vec![],
                    sections: vec![],
                    changelog: vec![],
                    signature: None,
                },
                clauses: vec![ClauseEntry {
                    spec: ClauseSpec {
                        clause_id: "C-TEST".to_string(),
                        title: "Clause".to_string(),
                        kind: ClauseKind::Normative,
                        status: ClauseStatus::Active,
                        text: "Clause text".to_string(),
                        anchors: vec![],
                        superseded_by: None,
                        since: None,
                        tags: vec![],
                    },
                    path: PathBuf::from("gov/rfc/RFC-0001/clauses/C-TEST.toml"),
                }],
                path: PathBuf::from("gov/rfc/RFC-0001/rfc.toml"),
            }],
            adrs: vec![],
            work_items: vec![
                work_item("WI-2026-06-07-001", "Alpha"),
                work_item("WI-2026-06-07-002", "Beta"),
            ],
        }
    }

    fn work_item(id: &str, title: &str) -> WorkItemEntry {
        WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta::new(id, title, WorkItemStatus::Active),
                content: WorkItemContent::default(),
                verification: WorkItemVerification::default(),
            },
            path: PathBuf::from(format!("gov/work/{id}.toml")),
        }
    }
}
