use super::super::app::App;
use crate::diagnostic::DiagnosticLevel;
use crate::status_counts::{count_by, counts_for_keys};
use crate::theme::{phase_semantic, status_icon, status_semantic};
use ratatui::{prelude::*, widgets::Paragraph};

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

    frame.render_widget(lifecycle_matrix(counts), top[0]);
    frame.render_widget(operations_panel(app, counts), top[1]);
    frame.render_widget(governance_index(app, true), rows[1]);
}

fn draw_compact(frame: &mut Frame, app: &App, area: Rect, counts: &DashboardCounts) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(6)])
        .split(area);

    let (state, state_color) = counts.system_state();
    let mut lines = lifecycle_lines(counts);
    lines.push(Line::from(vec![
        Span::styled("OPS    ", Style::default().fg(Color::Cyan).bold()),
        Span::raw(format!(
            "guards {}  loops {}  errors {}  warnings {}  ",
            app.supplement.guards.len(),
            app.supplement.loops.len(),
            counts.errors,
            counts.warnings
        )),
        Span::styled(state, Style::default().fg(state_color).bold()),
    ]));

    frame.render_widget(
        Paragraph::new(lines).block(
            super::panel_block("LIFECYCLE MATRIX").border_style(Style::default().fg(Color::Cyan)),
        ),
        rows[0],
    );
    frame.render_widget(governance_index(app, false), rows[1]);
}

fn lifecycle_matrix(counts: &DashboardCounts) -> Paragraph<'static> {
    let mut lines = vec![Line::from(vec![
        Span::styled("DOMAIN ", Style::default().fg(Color::DarkGray).bold()),
        Span::styled(
            "CURRENT GOVERNANCE STATE",
            Style::default().fg(Color::DarkGray),
        ),
    ])];
    lines.extend(lifecycle_lines(counts));

    Paragraph::new(lines).block(
        super::panel_block("LIFECYCLE MATRIX").border_style(Style::default().fg(Color::Cyan)),
    )
}

fn lifecycle_lines(counts: &DashboardCounts) -> Vec<Line<'static>> {
    vec![
        status_line(
            "RFC",
            &[
                ("draft", counts.rfc[0]),
                ("normative", counts.rfc[1]),
                ("deprecated", counts.rfc[2]),
            ],
        ),
        status_line(
            "ADR",
            &[
                ("proposed", counts.adr[0]),
                ("accepted", counts.adr[1]),
                ("rejected", counts.adr[2]),
                ("superseded", counts.adr[3]),
            ],
        ),
        status_line(
            "WORK",
            &[
                ("queue", counts.work[0]),
                ("active", counts.work[1]),
                ("done", counts.work[2]),
                ("cancelled", counts.work[3]),
            ],
        ),
        phase_line(&[
            ("spec", counts.phase[0]),
            ("impl", counts.phase[1]),
            ("test", counts.phase[2]),
            ("stable", counts.phase[3]),
        ]),
    ]
}

fn status_line(label: &'static str, statuses: &[(&'static str, usize)]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{label:<7}"),
        Style::default().fg(Color::Cyan).bold(),
    )];
    for (status, count) in statuses {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            status_icon(status),
            Style::default().fg(status_semantic(status).to_ratatui()),
        ));
        spans.push(Span::raw(format!(" {status} {count}")));
    }
    Line::from(spans)
}

fn phase_line(phases: &[(&'static str, usize)]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        "PHASE  ",
        Style::default().fg(Color::Cyan).bold(),
    )];
    for (phase, count) in phases {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "◆",
            Style::default().fg(phase_semantic(phase).to_ratatui()),
        ));
        spans.push(Span::raw(format!(" {phase} {count}")));
    }
    Line::from(spans)
}

fn operations_panel(app: &App, counts: &DashboardCounts) -> Paragraph<'static> {
    let (state, state_color) = counts.system_state();
    let lines = vec![
        operation_line("SYSTEM", state, state_color),
        operation_count("GUARDS", app.supplement.guards.len(), Color::LightBlue),
        operation_count("LOOPS", app.supplement.loops.len(), Color::Yellow),
        operation_count("RELEASES", app.supplement.releases.len(), Color::Cyan),
        operation_count("ERRORS", counts.errors, Color::Red),
        operation_count("WARNINGS", counts.warnings, Color::Yellow),
    ];

    Paragraph::new(lines)
        .block(super::panel_block("OPERATIONS").border_style(Style::default().fg(Color::DarkGray)))
}

fn operation_count(label: &'static str, value: usize, color: Color) -> Line<'static> {
    operation_line(label, &format!("{value:04}"), color)
}

fn operation_line(label: &'static str, value: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<10}"), Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(color).bold()),
    ])
}

fn governance_index(app: &App, wide: bool) -> Paragraph<'static> {
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
    let lines = entries
        .into_iter()
        .map(|(left, right)| index_pair(left, right, wide))
        .collect::<Vec<_>>();

    Paragraph::new(lines).block(
        super::panel_block("GOVERNANCE INDEX").border_style(Style::default().fg(Color::DarkGray)),
    )
}

fn index_pair(
    left: (&'static str, &'static str, usize, &'static str),
    right: (&'static str, &'static str, usize, &'static str),
    wide: bool,
) -> Line<'static> {
    let mut spans = index_entry(left, wide);
    spans.push(Span::raw(if wide { "    " } else { "  " }));
    spans.extend(index_entry(right, wide));
    Line::from(spans)
}

fn index_entry(
    entry: (&'static str, &'static str, usize, &'static str),
    wide: bool,
) -> Vec<Span<'static>> {
    let (key, label, count, hint) = entry;
    let mut spans = vec![
        Span::styled(format!("[{key}]"), Style::default().fg(Color::Cyan).bold()),
        Span::raw(format!(" {label:<14} {count:>4}")),
    ];
    if wide {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));
    }
    spans
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
