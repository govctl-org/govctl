use super::super::app::{App, View};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Borders, Paragraph},
};

fn breadcrumb(app: &App) -> String {
    match app.view {
        View::Dashboard => "Dashboard".to_string(),
        View::RfcList => "Dashboard > RFCs".to_string(),
        View::AdrList => "Dashboard > ADRs".to_string(),
        View::WorkList => "Dashboard > Work".to_string(),
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
            "RFC {} | ADR {} | Work {}",
            app.index.rfcs.len(),
            app.index.adrs.len(),
            app.index.work_items.len()
        ),
        View::RfcList | View::AdrList | View::WorkList => {
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
            parts.join(" | ")
        }
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
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(30)])
            .split(inner);

        let left = Paragraph::new(Line::from(vec![
            Span::styled("govctl", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" "),
            Span::raw(breadcrumb(app)),
        ]))
        .alignment(Alignment::Left);

        let right = Paragraph::new(header_status(app)).alignment(Alignment::Right);

        frame.render_widget(left, chunks[0]);
        frame.render_widget(right, chunks[1]);
    }
}

fn bindings_for_view(view: View) -> &'static [&'static str] {
    match view {
        View::Dashboard => &[
            "1/r", "RFCs", "2/a", "ADRs", "3/w", "Work", "?", "Help", "q", "Quit",
        ],
        View::RfcList | View::AdrList | View::WorkList => &[
            "j/k", "Navigate", "Enter", "View", "Esc", "Back", "/", "Filter", "g/G", "Jump", "?",
            "Help", "q", "Quit",
        ],
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
        View::AdrDetail(_) | View::WorkDetail(_) | View::ClauseDetail(_, _) => &[
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
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(30)])
            .split(inner);

        let left =
            Paragraph::new(keybind_line(bindings_for_view(self.view))).alignment(Alignment::Center);
        let right = Paragraph::new(self.status.unwrap_or("")).alignment(Alignment::Right);

        frame.render_widget(left, chunks[0]);
        frame.render_widget(right, chunks[1]);
    }
}
