use super::super::app::App;
use super::components::{SummaryCard, SummaryMetric};
use crate::status_counts::{count_by, counts_for_keys};
use ratatui::{prelude::*, widgets::Paragraph};

pub(super) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);

    let summary_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(rows[0]);

    frame.render_widget(rfc_stats(app), summary_chunks[0]);
    frame.render_widget(adr_stats(app), summary_chunks[1]);
    frame.render_widget(work_stats(app), summary_chunks[2]);
    frame.render_widget(ops_stats(app), summary_chunks[3]);
    frame.render_widget(cockpit_menu(app), rows[1]);
}

fn summary_block(
    title: &'static str,
    border_color: Color,
    metrics: Vec<SummaryMetric>,
    total: usize,
) -> Paragraph<'static> {
    SummaryCard::new(title, border_color, metrics, total).into_paragraph()
}

fn rfc_stats(app: &App) -> Paragraph<'static> {
    let counts = count_by(&app.index.rfcs, |rfc| rfc.rfc.status.as_ref());
    let counts = counts_for_keys(&counts, ["draft", "normative", "deprecated"]);

    summary_block(
        "📋 RFCs",
        Color::Blue,
        vec![
            SummaryMetric::new("○", Color::Yellow, "Draft:      ", counts[0]),
            SummaryMetric::new("●", Color::Green, "Normative:  ", counts[1]),
            SummaryMetric::new("✗", Color::Red, "Deprecated: ", counts[2]),
        ],
        app.index.rfcs.len(),
    )
}

fn adr_stats(app: &App) -> Paragraph<'static> {
    let counts = count_by(&app.index.adrs, |adr| adr.meta().status.as_ref());
    let counts = counts_for_keys(&counts, ["proposed", "accepted", "superseded"]);

    summary_block(
        "📝 ADRs",
        Color::Green,
        vec![
            SummaryMetric::new("○", Color::Yellow, "Proposed:   ", counts[0]),
            SummaryMetric::new("●", Color::Green, "Accepted:   ", counts[1]),
            SummaryMetric::new("✗", Color::Red, "Superseded: ", counts[2]),
        ],
        app.index.adrs.len(),
    )
}

fn work_stats(app: &App) -> Paragraph<'static> {
    let counts = count_by(&app.index.work_items, |item| item.meta().status.as_ref());
    let counts = counts_for_keys(&counts, ["queue", "active", "done"]);

    summary_block(
        "📌 Work Items",
        Color::Yellow,
        vec![
            SummaryMetric::new("○", Color::Yellow, "Queue:  ", counts[0]),
            SummaryMetric::new("◉", Color::Green, "Active: ", counts[1]),
            SummaryMetric::new("●", Color::Green, "Done:   ", counts[2]),
        ],
        app.index.work_items.len(),
    )
}

fn ops_stats(app: &App) -> Paragraph<'static> {
    let error_count = app
        .supplement
        .diagnostics
        .iter()
        .filter(|diag| diag.level == crate::diagnostic::DiagnosticLevel::Error)
        .count();
    let warning_count = app
        .supplement
        .diagnostics
        .iter()
        .filter(|diag| diag.level == crate::diagnostic::DiagnosticLevel::Warning)
        .count();
    // Diagnostics are shown below as an error/warning breakdown, not as primary Ops items.
    let total = app.supplement.guards.len() + app.supplement.loops.len();

    summary_block(
        "Ops",
        Color::Cyan,
        vec![
            SummaryMetric::new(
                "G",
                Color::LightBlue,
                "Guards: ",
                app.supplement.guards.len(),
            ),
            SummaryMetric::new("L", Color::Yellow, "Loops:  ", app.supplement.loops.len()),
            SummaryMetric::new("!", Color::Red, "Errors: ", error_count),
            SummaryMetric::new("?", Color::Yellow, "Warns:  ", warning_count),
        ],
        total,
    )
}

fn cockpit_menu(app: &App) -> Paragraph<'static> {
    let lines = vec![
        Line::from(vec![
            Span::styled("Read-only cockpit", Style::default().fg(Color::Cyan).bold()),
            Span::raw("  "),
            Span::styled("RFC-0007", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        menu_line("r", "RFCs", app.index.rfcs.len(), "governance requirements"),
        menu_line(
            "c",
            "Clauses",
            app.supplement.clauses.len(),
            "normative obligations",
        ),
        menu_line("a", "ADRs", app.index.adrs.len(), "accepted decisions"),
        menu_line("w", "Work", app.index.work_items.len(), "planned outcomes"),
        menu_line(
            "g",
            "Guards",
            app.supplement.guards.len(),
            "verification commands",
        ),
        menu_line(
            "9",
            "Releases",
            app.supplement.releases.len(),
            "released artifacts",
        ),
        menu_line("t", "Tags", app.supplement.tags.len(), "repository tags"),
        menu_line(
            "s",
            "Search",
            app.search_results.len(),
            "project-wide discovery",
        ),
        menu_line(
            "l",
            "Loops",
            app.supplement.loops.len(),
            "local execution state",
        ),
        menu_line(
            "d",
            "Diagnostics",
            app.supplement.diagnostics.len(),
            "check findings",
        ),
    ];
    Paragraph::new(lines)
        .block(super::rounded_block("Cockpit").border_style(Style::default().fg(Color::DarkGray)))
}

fn menu_line(
    key: &'static str,
    label: &'static str,
    count: usize,
    hint: &'static str,
) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("[{key}]"), Style::default().fg(Color::Cyan).bold()),
        Span::raw(" "),
        Span::styled(format!("{label:<12}"), Style::default().bold()),
        Span::styled(format!("{count:>4}"), Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(hint, Style::default().fg(Color::DarkGray)),
    ])
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
