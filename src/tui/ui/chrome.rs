use super::super::app::{App, View};
use super::components::ChromeBar;
use super::panel_block;
use crate::diagnostic::DiagnosticLevel;
use ratatui::{prelude::*, widgets::LineGauge};

pub(super) fn shows_command_strip(view: View) -> bool {
    view.is_list()
}

fn breadcrumb(app: &App) -> String {
    if let Some(label) = app.view.list_label() {
        return format!("Dashboard > {label}");
    }
    match app.view {
        View::Dashboard => "Dashboard".to_string(),
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
        View::ConformanceDetail(idx) => app
            .index
            .conformance_cases
            .get(idx)
            .map(|case| format!("Dashboard > Cases > {}", case.meta().id))
            .unwrap_or_else(|| "Dashboard > Cases".to_string()),
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
        _ => unreachable!("list views return before breadcrumb detail dispatch"),
    }
}

fn header_status(app: &mut App) -> String {
    match app.view {
        View::Dashboard => {
            let errors = app
                .supplement
                .diagnostics
                .iter()
                .filter(|diag| diag.level == DiagnosticLevel::Error)
                .count();
            let warnings = app
                .supplement
                .diagnostics
                .iter()
                .filter(|diag| diag.level == DiagnosticLevel::Warning)
                .count();
            let state = if errors > 0 {
                "ERROR"
            } else if warnings > 0 {
                "WARN"
            } else {
                "NOMINAL"
            };
            format!(
                "v{} | {} | E {} | W {}",
                env!("CARGO_PKG_VERSION"),
                state,
                errors,
                warnings
            )
        }
        view if view.is_list() => {
            let shown = app.list_len();
            if shown > 0 {
                format!("SEL {} / {}", app.selected + 1, shown)
            } else {
                "SEL 0 / 0".to_string()
            }
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

pub(super) struct CommandStrip<'a> {
    app: &'a mut App,
}

impl<'a> CommandStrip<'a> {
    pub(super) fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub(super) fn render(self, frame: &mut Frame, area: Rect) {
        let app = self.app;
        let shown = app.list_len();
        let total = app.list_total_len();
        let (label, value, focused) = if app.view == View::Search {
            ("QUERY", app.search_query.as_str(), app.search_mode)
        } else {
            ("FILTER", app.filter_query.as_str(), app.filter_mode)
        };
        let focus_label = if focused { "EDITING" } else { "READY" };
        let border_color = if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        if area.width < 60 {
            let title = format!("{label} // {focus_label}");
            let block = panel_block(&title)
                .title_bottom(
                    Line::from(format!(" {shown} / {total} "))
                        .right_aligned()
                        .style(Style::default().fg(Color::DarkGray)),
                )
                .border_style(Style::default().fg(border_color));
            let inner_width = block.inner(area).width as usize;
            frame.render_widget(
                ratatui::widgets::Paragraph::new(input_line(value, focused, inner_width))
                    .block(block),
                area,
            );
            return;
        }

        let match_width = (area.width / 4).clamp(20, 28);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(24), Constraint::Length(match_width)])
            .split(area);
        let title = format!("{label} // {focus_label}");
        let input_block = panel_block(&title).border_style(Style::default().fg(border_color));
        let input_width = input_block.inner(chunks[0]).width as usize;
        frame.render_widget(
            ratatui::widgets::Paragraph::new(input_line(value, focused, input_width))
                .block(input_block),
            chunks[0],
        );

        let ratio = if total == 0 {
            0.0
        } else {
            shown as f64 / total as f64
        };
        let gauge = LineGauge::default()
            .block(panel_block("MATCH SET").border_style(Style::default().fg(Color::DarkGray)))
            .label(format!("{shown} / {total}"))
            .ratio(ratio)
            .filled_symbol("━")
            .unfilled_symbol("─")
            .filled_style(Style::default().fg(Color::Cyan))
            .unfilled_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(gauge, chunks[1]);
    }
}

fn input_line(value: &str, focused: bool, max_width: usize) -> Line<'static> {
    let cursor_width = usize::from(focused);
    let value_width = max_width.saturating_sub(2 + cursor_width);
    let visible = tail_with_ellipsis(value, value_width);
    let mut spans = vec![
        Span::styled("/", Style::default().fg(Color::Cyan).bold()),
        Span::raw(visible),
    ];
    if focused {
        spans.push(Span::styled("▏", Style::default().fg(Color::Cyan).bold()));
    }
    Line::from(spans)
}

fn tail_with_ellipsis(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if Line::from(value).width() <= max_width {
        return value.to_string();
    }

    let mut start = value.len();
    let mut used = 1;
    for (idx, ch) in value.char_indices().rev() {
        let char_width = Span::raw(ch.to_string()).width();
        if used + char_width > max_width {
            break;
        }
        start = idx;
        used += char_width;
    }
    format!("…{}", &value[start..])
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
        let control_plane = (area.width >= 90).then_some(Span::styled(
            " // CONTROL PLANE",
            Style::default().fg(Color::DarkGray),
        ));
        let left = Line::from(vec![
            Span::styled("GOVCTL", Style::default().fg(Color::Cyan).bold()),
            control_plane.unwrap_or_else(|| Span::raw("")),
            Span::styled(" // ", Style::default().fg(Color::DarkGray)),
            Span::raw(breadcrumb(app).to_uppercase()),
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
            "x",
            "Cases",
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
        view if view.is_standard_list() && view.selection_opens_detail() => &[
            "j/k", "Navigate", "Enter", "View", "Esc", "Back", "/", "Filter", "g/G", "Jump", "?",
            "Help", "q", "Quit",
        ],
        view if view.is_standard_list() => &[
            "j/k", "Navigate", "Esc", "Back", "/", "Filter", "g/G", "Jump", "?", "Help", "q",
            "Quit",
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
        | View::ConformanceDetail(_)
        | View::ClauseDetail(_, _) => &[
            "j/k", "Scroll", "^d/^u", "Page", "Esc", "Back", "?", "Help", "q", "Quit",
        ],
        _ => unreachable!("every view has a declared footer binding group"),
    }
}

fn compact_bindings_for_view(view: View) -> &'static [&'static str] {
    match view {
        View::Dashboard => &[
            "r", "RFC", "c", "Clause", "x", "Case", "s", "Search", "?", "Help", "q", "Quit",
        ],
        view if view.is_standard_list() && view.selection_opens_detail() => &[
            "j/k", "Move", "Enter", "Open", "Esc", "Back", "/", "Filter", "?", "Help", "q", "Quit",
        ],
        view if view.is_standard_list() => &[
            "j/k", "Move", "Esc", "Back", "/", "Filter", "?", "Help", "q", "Quit",
        ],
        View::Search => &[
            "e//", "Query", "Enter", "Open", "j/k", "Move", "Esc", "Back", "?", "Help", "q", "Quit",
        ],
        View::LoopDetail(_) => &["j/k", "Select", "Esc", "Back", "?", "Help", "q", "Quit"],
        View::RfcDetail(_) => &[
            "j/k", "Clause", "Enter", "Open", "Esc", "Back", "?", "Help", "q", "Quit",
        ],
        View::AdrDetail(_)
        | View::WorkDetail(_)
        | View::GuardDetail(_)
        | View::ConformanceDetail(_)
        | View::ClauseDetail(_, _) => &[
            "j/k", "Scroll", "^d/^u", "Page", "Esc", "Back", "?", "Help", "q", "Quit",
        ],
        _ => unreachable!("every view has a declared compact footer binding group"),
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
        let bindings = if area.width < 90 {
            compact_bindings_for_view(self.view)
        } else {
            bindings_for_view(self.view)
        };
        ChromeBar::new(
            Color::DarkGray,
            keybind_line(bindings),
            self.status.unwrap_or(""),
        )
        .left_alignment(Alignment::Center)
        .render(frame, area);
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        adr, clause, conformance_case, project_index, render_app, rfc, work_item,
    };
    use super::*;
    use crate::loop_state::LoopState;
    use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
    use crate::tui::data::TuiLoopEntry;
    use std::collections::BTreeMap;

    #[test]
    fn breadcrumb_covers_primary_and_detail_views() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = chrome_app()?;

        for (view, expected) in [
            (View::Dashboard, "Dashboard"),
            (View::RfcList, "Dashboard > RFCs"),
            (View::ClauseList, "Dashboard > Clauses"),
            (View::AdrList, "Dashboard > ADRs"),
            (View::WorkList, "Dashboard > Work"),
            (View::GuardList, "Dashboard > Guards"),
            (View::ConformanceList, "Dashboard > Cases"),
            (View::ReleaseList, "Dashboard > Releases"),
            (View::TagList, "Dashboard > Tags"),
            (View::Search, "Dashboard > Search"),
            (View::LoopList, "Dashboard > Loops"),
            (View::DiagnosticList, "Dashboard > Diagnostics"),
            (View::RfcDetail(0), "Dashboard > RFCs > RFC-0001"),
            (View::AdrDetail(0), "Dashboard > ADRs > ADR-0001"),
            (View::WorkDetail(0), "Dashboard > Work > WI-2026-01-01-001"),
            (
                View::ConformanceDetail(0),
                "Dashboard > Cases > CONF-CHROME",
            ),
            (
                View::ClauseDetail(0, 0),
                "Dashboard > RFCs > RFC-0001 > C-TEST",
            ),
            (
                View::LoopDetail(0),
                "Dashboard > Loops > LOOP-2026-06-07-001",
            ),
        ] {
            app.view = view;
            assert_eq!(breadcrumb(&app), expected);
        }
        Ok(())
    }

    #[test]
    fn header_status_covers_dashboard_list_search_and_loop()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = chrome_app()?;

        assert!(header_status(&mut app).contains("NOMINAL"));

        app.go_to(View::RfcList);
        assert!(header_status(&mut app).contains("SEL 1 / 1"));
        app.enter_filter_mode();
        app.filter_query = "rfc".to_string();
        assert!(header_status(&mut app).contains("SEL 1 / 1"));

        app.go_to(View::Search);
        app.search_query = "work".to_string();
        app.enter_search_mode();
        assert!(header_status(&mut app).contains("SEL 0 / 0"));

        app.view = View::LoopDetail(0);
        assert!(header_status(&mut app).contains("start"));
        app.view = View::LoopDetail(99);
        assert_eq!(header_status(&mut app), "invalid loop state");
        Ok(())
    }

    #[test]
    fn command_strip_preserves_filter_focus_and_long_input_tail()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = chrome_app()?;
        app.go_to(View::RfcList);
        app.enter_filter_mode();
        app.filter_query =
            "a-very-long-filter-value-that-must-keep-its-visible-tail-value".to_string();

        let (_, rendered) = render_app(72, 3, app, |frame, app| {
            CommandStrip::new(app).render(frame, frame.area());
        })?;

        assert!(
            rendered
                .iter()
                .any(|line| line.contains("FILTER // EDITING"))
        );
        assert!(rendered.iter().any(|line| line.contains("…")));
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("visible-tail-value▏"))
        );
        assert!(rendered.iter().any(|line| line.contains("MATCH SET")));
        Ok(())
    }

    #[test]
    fn narrow_empty_filter_keeps_complete_label_and_cursor()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = chrome_app()?;
        app.go_to(View::RfcList);
        app.enter_filter_mode();

        let (_, rendered) = render_app(50, 3, app, |frame, app| {
            CommandStrip::new(app).render(frame, frame.area());
        })?;

        assert!(
            rendered
                .iter()
                .any(|line| line.contains("FILTER // EDITING"))
        );
        assert!(rendered.iter().any(|line| line.contains("/▏")));
        assert!(rendered.iter().any(|line| line.contains("1 / 1")));
        Ok(())
    }

    #[test]
    fn input_tail_uses_available_display_width() {
        assert_eq!(
            tail_with_ellipsis("abcdefghijklmnopqrstuvwxyz", 8),
            "…tuvwxyz"
        );
        assert_eq!(tail_with_ellipsis("short", 8), "short");
        assert_eq!(tail_with_ellipsis("anything", 0), "");
    }

    #[test]
    fn footer_bindings_cover_all_view_groups() {
        for view in [
            View::Dashboard,
            View::RfcList,
            View::Search,
            View::LoopDetail(0),
            View::RfcDetail(0),
            View::WorkDetail(0),
            View::ConformanceDetail(0),
        ] {
            let line = keybind_line(bindings_for_view(view));
            assert!(line.width() > 0);
            let compact = keybind_line(compact_bindings_for_view(view));
            assert!(compact.width() > 0);
        }
    }

    #[test]
    fn header_and_footer_render_visible_chrome() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = chrome_app()?;
        app.view = View::RfcList;

        let (_, rendered) = render_app(100, 6, app, |frame, app| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3)])
                .split(frame.area());
            Header::new(app).render(frame, chunks[0]);
            Footer::new(app.view, Some("status")).render(frame, chunks[1]);
        })?;

        assert!(rendered.iter().any(|line| line.contains("GOVCTL")));
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("DASHBOARD > RFCS"))
        );
        assert!(rendered.iter().any(|line| line.contains("status")));
        Ok(())
    }

    #[test]
    fn narrow_header_preserves_brand_and_breadcrumb() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = chrome_app()?;
        app.view = View::RfcList;

        let (_, rendered) = render_app(72, 3, app, |frame, app| {
            Header::new(app).render(frame, frame.area());
        })?;

        assert!(rendered.iter().any(|line| line.contains("GOVCTL")));
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("DASHBOARD > RFCS"))
        );
        Ok(())
    }

    fn chrome_app() -> Result<App, Box<dyn std::error::Error>> {
        let mut rfc = rfc(
            "RFC-0001",
            "RFC title",
            RfcStatus::Normative,
            RfcPhase::Impl,
            &[],
        );
        rfc.clauses
            .push(clause("C-TEST", "Clause title", "Clause text"));
        let mut index = project_index(
            vec![rfc],
            vec![adr("ADR-0001", "ADR title", AdrStatus::Accepted, &[])],
            vec![work_item(
                "WI-2026-01-01-001",
                "Work title",
                WorkItemStatus::Active,
                &[],
            )],
        );
        index.conformance_cases.push(conformance_case(
            "CONF-CHROME",
            "Chrome case",
            "RFC-0001:C-TEST",
            "0.1.0",
            &[],
        ));
        let mut app = App::new(index);

        let work_id = "WI-2026-06-07-001";
        let mut dependencies = BTreeMap::new();
        dependencies.insert(work_id.to_string(), Vec::new());
        app.supplement.loops.push(TuiLoopEntry {
            id: "LOOP-2026-06-07-001".to_string(),
            state: Some(LoopState::new(
                "LOOP-2026-06-07-001",
                vec![work_id.to_string()],
                vec![work_id.to_string()],
                dependencies,
            )?),
            diagnostic: None,
        });
        Ok(app)
    }
}
