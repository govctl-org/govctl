use super::super::app::{App, View};
use super::components::ChromeBar;
use ratatui::prelude::*;

fn breadcrumb(app: &App) -> String {
    match app.view {
        View::Dashboard => "Dashboard".to_string(),
        View::RfcList => "Dashboard > RFCs".to_string(),
        View::ClauseList => "Dashboard > Clauses".to_string(),
        View::AdrList => "Dashboard > ADRs".to_string(),
        View::WorkList => "Dashboard > Work".to_string(),
        View::GuardList => "Dashboard > Guards".to_string(),
        View::ReleaseList => "Dashboard > Releases".to_string(),
        View::TagList => "Dashboard > Tags".to_string(),
        View::Search => "Dashboard > Search".to_string(),
        View::LoopList => "Dashboard > Loops".to_string(),
        View::LoopDetail(idx) => app
            .supplement
            .loops
            .get(idx)
            .map(|entry| format!("Dashboard > Loops > {}", entry.id))
            .unwrap_or_else(|| "Dashboard > Loops".to_string()),
        View::DiagnosticList => "Dashboard > Diagnostics".to_string(),
        View::RfcDetail(idx) => app
            .index
            .rfcs
            .get(idx)
            .map(|rfc| format!("Dashboard > RFCs > {}", rfc.rfc.rfc_id))
            .unwrap_or_else(|| "Dashboard > RFCs".to_string()),
        View::AdrDetail(idx) => app
            .index
            .adrs
            .get(idx)
            .map(|adr| format!("Dashboard > ADRs > {}", adr.meta().id))
            .unwrap_or_else(|| "Dashboard > ADRs".to_string()),
        View::WorkDetail(idx) => app
            .index
            .work_items
            .get(idx)
            .map(|item| format!("Dashboard > Work > {}", item.meta().id))
            .unwrap_or_else(|| "Dashboard > Work".to_string()),
        View::GuardDetail(idx) => app
            .supplement
            .guards
            .get(idx)
            .map(|guard| format!("Dashboard > Guards > {}", guard.meta().id))
            .unwrap_or_else(|| "Dashboard > Guards".to_string()),
        View::ClauseDetail(rfc_idx, clause_idx) => app
            .index
            .rfcs
            .get(rfc_idx)
            .and_then(|rfc| rfc.clauses.get(clause_idx).map(|clause| (rfc, clause)))
            .map(|(rfc, clause)| {
                format!(
                    "Dashboard > RFCs > {} > {}",
                    rfc.rfc.rfc_id, clause.spec.clause_id
                )
            })
            .unwrap_or_else(|| "Dashboard > RFCs".to_string()),
    }
}

fn header_status(app: &mut App) -> String {
    match app.view {
        View::Dashboard => format!(
            "RFC {} | ADR {} | Work {} | Guard {} | Loop {} | Diag {}",
            app.index.rfcs.len(),
            app.index.adrs.len(),
            app.index.work_items.len(),
            app.supplement.guards.len(),
            app.supplement.loops.len(),
            app.supplement.diagnostics.len()
        ),
        View::RfcList
        | View::ClauseList
        | View::AdrList
        | View::WorkList
        | View::GuardList
        | View::ReleaseList
        | View::TagList
        | View::Search
        | View::LoopList
        | View::DiagnosticList => {
            let total = app.list_total_len();
            let shown = app.list_len();
            let mut parts = vec![format!("Shown {}/{}", shown, total)];
            if shown > 0 {
                parts.push(format!("Sel {}/{}", app.selected + 1, shown));
            }
            if app.filter_mode {
                parts.push(format!("Filter: /{}_", app.filter_query));
            } else if app.filter_active() {
                parts.push(format!("Filter: {}", app.filter_query));
            }
            if app.view == View::Search {
                if app.search_mode {
                    parts.push(format!("Query: {}_", app.search_query));
                } else if !app.search_query.is_empty() {
                    parts.push(format!("Query: {}", app.search_query));
                }
            }
            parts.join(" | ")
        }
        View::LoopDetail(idx) => app
            .current_loop_state(idx)
            .map(|state| {
                format!(
                    "{} | round {} | {}",
                    state.loop_meta.state.as_str(),
                    state.loop_meta.current_round,
                    state.loop_meta.next_action.as_str()
                )
            })
            .unwrap_or_else(|| "invalid loop state".to_string()),
        _ => String::new(),
    }
}

pub(super) struct Header<'a> {
    app: &'a mut App,
}

impl<'a> Header<'a> {
    pub(super) fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    // Implements [[RFC-0003:C-NAV]]
    pub(super) fn render(self, frame: &mut Frame, area: Rect) {
        let app = self.app;
        let left = Line::from(vec![
            Span::styled("govctl", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" "),
            Span::raw(breadcrumb(app)),
        ]);

        ChromeBar::new(Color::Cyan, left, header_status(app)).render(frame, area);
    }
}

fn bindings_for_view(view: View) -> &'static [&'static str] {
    match view {
        View::Dashboard => &[
            "r",
            "RFCs",
            "c",
            "Clauses",
            "a",
            "ADRs",
            "w",
            "Work",
            "s",
            "Search",
            "l",
            "Loops",
            "d",
            "Diagnostics",
            "?",
            "Help",
            "q",
            "Quit",
        ],
        View::RfcList
        | View::ClauseList
        | View::AdrList
        | View::WorkList
        | View::GuardList
        | View::ReleaseList
        | View::TagList
        | View::LoopList
        | View::DiagnosticList => &[
            "j/k", "Navigate", "Enter", "View", "Esc", "Back", "/", "Filter", "g/G", "Jump", "?",
            "Help", "q", "Quit",
        ],
        View::Search => &[
            "e//",
            "Edit Query",
            "Enter",
            "View",
            "j/k",
            "Navigate",
            "Esc",
            "Back",
            "?",
            "Help",
            "q",
            "Quit",
        ],
        View::LoopDetail(_) => &["j/k", "Select", "Esc", "Back", "?", "Help", "q", "Quit"],
        View::RfcDetail(_) => &[
            "j/k",
            "Navigate",
            "Enter",
            "View Clause",
            "Esc",
            "Back",
            "?",
            "Help",
            "q",
            "Quit",
        ],
        View::AdrDetail(_)
        | View::WorkDetail(_)
        | View::GuardDetail(_)
        | View::ClauseDetail(_, _) => &[
            "j/k", "Scroll", "^d/^u", "Page", "Esc", "Back", "?", "Help", "q", "Quit",
        ],
    }
}

fn keybind_line(bindings: &[&str]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for chunk in bindings.chunks(2) {
        if chunk.len() == 2 {
            spans.push(Span::styled("[", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                chunk[0].to_string(),
                Style::default().fg(Color::Cyan).bold(),
            ));
            spans.push(Span::styled("] ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("{}  ", chunk[1]),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
    Line::from(spans)
}

pub(super) struct Footer<'a> {
    view: View,
    status: Option<&'a str>,
}

impl<'a> Footer<'a> {
    pub(super) fn new(view: View, status: Option<&'a str>) -> Self {
        Self { view, status }
    }

    // Implements [[RFC-0003:C-NAV]]
    pub(super) fn render(self, frame: &mut Frame, area: Rect) {
        ChromeBar::new(
            Color::DarkGray,
            keybind_line(bindings_for_view(self.view)),
            self.status.unwrap_or(""),
        )
        .left_alignment(Alignment::Center)
        .render(frame, area);
    }
}
