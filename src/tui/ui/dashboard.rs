use super::super::app::App;
use crate::diagnostic::DiagnosticLevel;
use crate::status_counts::{count_by, counts_for_keys};
use crate::theme::{phase_semantic, status_icon, status_semantic};
use ratatui::{
    prelude::*,
    widgets::{Cell, Paragraph, Row, Table},
};

struct DashboardCounts {
    rfc: [usize; 3],
    adr: [usize; 4],
    work: [usize; 4],
    phase: [usize; 4],
    errors: usize,
    warnings: usize,
}

impl DashboardCounts {
    fn from_app(app: &App) -> Self {
        let rfc = count_by(&app.index.rfcs, |rfc| rfc.rfc.status.as_ref());
        let adr = count_by(&app.index.adrs, |adr| adr.meta().status.as_ref());
        let work = count_by(&app.index.work_items, |item| item.meta().status.as_ref());
        let phase = count_by(&app.index.rfcs, |rfc| rfc.rfc.phase.as_ref());

        Self {
            rfc: counts_for_keys(&rfc, ["draft", "normative", "deprecated"]),
            adr: counts_for_keys(&adr, ["proposed", "accepted", "rejected", "superseded"]),
            work: counts_for_keys(&work, ["queue", "active", "done", "cancelled"]),
            phase: counts_for_keys(&phase, ["spec", "impl", "test", "stable"]),
            errors: app
                .supplement
                .diagnostics
                .iter()
                .filter(|diag| diag.level == DiagnosticLevel::Error)
                .count(),
            warnings: app
                .supplement
                .diagnostics
                .iter()
                .filter(|diag| diag.level == DiagnosticLevel::Warning)
                .count(),
        }
    }

    fn system_state(&self) -> (&'static str, Color) {
        if self.errors > 0 {
            ("ERROR", Color::Red)
        } else if self.warnings > 0 {
            ("WARN", Color::Yellow)
        } else {
            ("NOMINAL", Color::Green)
        }
    }
}

pub(super) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let counts = DashboardCounts::from_app(app);
    if area.width < 100 {
        draw_compact(frame, app, area, &counts);
    } else {
        draw_wide(frame, app, area, &counts);
    }
}

fn draw_wide(frame: &mut Frame, app: &App, area: Rect, counts: &DashboardCounts) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(7)])
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(rows[0]);
    let control = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(5)])
        .split(top[1]);

    frame.render_widget(lifecycle_matrix(counts, false), top[0]);
    frame.render_widget(execution_panel(app), control[0]);
    frame.render_widget(health_panel(counts, false), control[1]);
    frame.render_widget(governance_index(app, false), rows[1]);
}

fn draw_compact(frame: &mut Frame, app: &App, area: Rect, counts: &DashboardCounts) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(4),
            Constraint::Min(7),
        ])
        .split(area);
    let control = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    frame.render_widget(lifecycle_matrix(counts, true), rows[0]);
    frame.render_widget(execution_panel(app), control[0]);
    frame.render_widget(health_panel(counts, true), control[1]);
    frame.render_widget(governance_index(app, true), rows[2]);
}

fn lifecycle_matrix(counts: &DashboardCounts, compact: bool) -> Table<'static> {
    let widths = if compact {
        vec![
            Constraint::Length(7),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(14),
            Constraint::Length(14),
        ]
    } else {
        vec![
            Constraint::Length(8),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(16),
        ]
    };
    let row_spacing = u16::from(!compact);
    let header = Row::new(["DOMAIN", "PENDING", "CURRENT", "CLOSED", "RETIRED"])
        .style(Style::default().fg(Color::DarkGray).bold())
        .bottom_margin(1);
    let rows = vec![
        lifecycle_row(
            "RFC",
            Some(("draft", counts.rfc[0])),
            Some(("normative", counts.rfc[1])),
            None,
            Some(("deprecated", counts.rfc[2])),
        )
        .bottom_margin(row_spacing),
        lifecycle_row(
            "ADR",
            Some(("proposed", counts.adr[0])),
            Some(("accepted", counts.adr[1])),
            Some(("rejected", counts.adr[2])),
            Some(("superseded", counts.adr[3])),
        )
        .bottom_margin(row_spacing),
        lifecycle_row(
            "WORK",
            Some(("queue", counts.work[0])),
            Some(("active", counts.work[1])),
            Some(("done", counts.work[2])),
            Some(("cancelled", counts.work[3])),
        ),
    ];
    let block = super::panel_block("LIFECYCLE MATRIX")
        .title_bottom(phase_summary(counts))
        .border_style(Style::default().fg(Color::Cyan));

    Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(block)
}

fn lifecycle_row(
    domain: &'static str,
    pending: Option<(&'static str, usize)>,
    current: Option<(&'static str, usize)>,
    closed: Option<(&'static str, usize)>,
    retired: Option<(&'static str, usize)>,
) -> Row<'static> {
    Row::new(vec![
        Cell::from(Line::styled(
            domain,
            Style::default().fg(Color::Cyan).bold(),
        )),
        lifecycle_cell(pending),
        lifecycle_cell(current),
        lifecycle_cell(closed),
        lifecycle_cell(retired),
    ])
}

fn lifecycle_cell(status: Option<(&'static str, usize)>) -> Cell<'static> {
    let Some((status, count)) = status else {
        return Cell::from(Line::styled("—", Style::default().fg(Color::DarkGray)));
    };
    let color = status_semantic(status).to_ratatui();
    Cell::from(Line::from(vec![
        Span::styled(status_icon(status), Style::default().fg(color)),
        Span::raw(format!(" {status} {count}")),
    ]))
}

fn phase_summary(counts: &DashboardCounts) -> Line<'static> {
    let phases = [
        ("spec", counts.phase[0]),
        ("impl", counts.phase[1]),
        ("test", counts.phase[2]),
        ("stable", counts.phase[3]),
    ];
    let mut spans = Vec::with_capacity(phases.len() * 2 + 2);
    spans.push(Span::raw(" "));
    for (idx, (phase, count)) in phases.into_iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            format!("{} {count}", phase.to_uppercase()),
            Style::default()
                .fg(phase_semantic(phase).to_ratatui())
                .bold(),
        ));
    }
    spans.push(Span::raw(" "));
    Line::from(spans).right_aligned()
}

fn execution_panel(app: &App) -> Paragraph<'static> {
    Paragraph::new(vec![
        metric_line("GUARDS", app.supplement.guards.len(), Color::LightBlue),
        metric_line("LOOPS", app.supplement.loops.len(), Color::Yellow),
    ])
    .block(super::panel_block("EXECUTION").border_style(Style::default().fg(Color::LightBlue)))
}

fn health_panel(counts: &DashboardCounts, compact: bool) -> Paragraph<'static> {
    let (state, state_color) = counts.system_state();
    let lines = if compact {
        vec![
            Line::from(Span::styled(state, Style::default().fg(state_color).bold())),
            Line::from(vec![
                Span::styled(
                    format!("E {:04}", counts.errors),
                    Style::default().fg(if counts.errors > 0 {
                        Color::Red
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("W {:04}", counts.warnings),
                    Style::default().fg(if counts.warnings > 0 {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    }),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("STATE     ", Style::default().fg(Color::DarkGray)),
                Span::styled(state, Style::default().fg(state_color).bold()),
            ]),
            metric_line(
                "ERRORS",
                counts.errors,
                if counts.errors > 0 {
                    Color::Red
                } else {
                    Color::DarkGray
                },
            ),
            metric_line(
                "WARNINGS",
                counts.warnings,
                if counts.warnings > 0 {
                    Color::Yellow
                } else {
                    Color::DarkGray
                },
            ),
        ]
    };

    Paragraph::new(lines)
        .block(super::panel_block("HEALTH").border_style(Style::default().fg(state_color)))
}

fn metric_line(label: &'static str, value: usize, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<10}"), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{value:04}"), Style::default().fg(color).bold()),
    ])
}

type IndexEntry = (&'static str, &'static str, usize, &'static str);

fn governance_index(app: &App, compact: bool) -> Table<'static> {
    let entries = [
        (
            ("r", "RFC INDEX", app.index.rfcs.len(), "requirements"),
            (
                "c",
                "CLAUSE INDEX",
                app.supplement.clauses.len(),
                "obligations",
            ),
        ),
        (
            ("a", "ADR INDEX", app.index.adrs.len(), "decisions"),
            ("w", "WORK QUEUE", app.index.work_items.len(), "outcomes"),
        ),
        (
            (
                "g",
                "GUARD MATRIX",
                app.supplement.guards.len(),
                "verification",
            ),
            ("9", "RELEASE LOG", app.supplement.releases.len(), "history"),
        ),
        (
            ("t", "TAG INDEX", app.supplement.tags.len(), "taxonomy"),
            ("s", "SEARCH", app.search_results.len(), "discovery"),
        ),
        (
            ("l", "LOOP CONTROL", app.supplement.loops.len(), "execution"),
            (
                "d",
                "DIAGNOSTICS",
                app.supplement.diagnostics.len(),
                "findings",
            ),
        ),
    ];

    let (rows, widths) = if compact {
        let rows: Vec<Row<'static>> = entries
            .into_iter()
            .map(|(left, right)| compact_index_row(left, right))
            .collect();
        (
            rows,
            vec![
                Constraint::Length(3),
                Constraint::Length(20),
                Constraint::Length(5),
                Constraint::Length(3),
                Constraint::Length(20),
                Constraint::Length(5),
            ],
        )
    } else {
        let rows: Vec<Row<'static>> = entries
            .into_iter()
            .map(|(left, right)| wide_index_row(left, right))
            .collect();
        (
            rows,
            vec![
                Constraint::Length(3),
                Constraint::Length(14),
                Constraint::Length(6),
                Constraint::Length(14),
                Constraint::Length(3),
                Constraint::Length(14),
                Constraint::Length(6),
                Constraint::Length(14),
            ],
        )
    };

    Table::new(rows, widths).column_spacing(2).block(
        super::panel_block("GOVERNANCE INDEX").border_style(Style::default().fg(Color::DarkGray)),
    )
}

fn compact_index_row(left: IndexEntry, right: IndexEntry) -> Row<'static> {
    Row::new(vec![
        index_key(left.0),
        index_label(left.1),
        index_count(left.2),
        index_key(right.0),
        index_label(right.1),
        index_count(right.2),
    ])
}

fn wide_index_row(left: IndexEntry, right: IndexEntry) -> Row<'static> {
    Row::new(vec![
        index_key(left.0),
        index_label(left.1),
        index_count(left.2),
        index_purpose(left.3),
        index_key(right.0),
        index_label(right.1),
        index_count(right.2),
        index_purpose(right.3),
    ])
}

fn index_key(key: &'static str) -> Cell<'static> {
    Cell::from(Line::styled(
        format!("[{key}]"),
        Style::default().fg(Color::Cyan).bold(),
    ))
}

fn index_label(label: &'static str) -> Cell<'static> {
    Cell::from(Line::styled(label, Style::default().bold()))
}

fn index_count(count: usize) -> Cell<'static> {
    Cell::from(
        Line::styled(format!("{count:04}"), Style::default().fg(Color::Yellow)).right_aligned(),
    )
}

fn index_purpose(purpose: &'static str) -> Cell<'static> {
    Cell::from(Line::styled(purpose, Style::default().fg(Color::DarkGray)))
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
