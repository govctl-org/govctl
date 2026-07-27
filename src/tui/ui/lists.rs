use super::super::app::App;
use super::components::{
    PhaseCell, ResourceListRow, ResourceTable, ResourceTableSpec, StatusText, TagsCell,
};
use super::panel_block;
use crate::diagnostic::DiagnosticLevel;
use crate::model::ConformanceApplicability;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Row, Wrap},
};

pub(super) fn draw_rfc(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.index.rfcs,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(10),
                Constraint::Min(20),
                Constraint::Length(14),
                Constraint::Length(10),
                Constraint::Min(15),
            ],
            compact_widths: vec![
                Constraint::Length(10),
                Constraint::Min(16),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(0),
            ],
            headers: &["ID", "Title", "Status", "Phase", "Tags"],
            header_color: Color::Cyan,
            title: "RFC INDEX",
            border_color: Color::Blue,
        },
        |rfc| {
            let status = rfc.rfc.status.as_ref();
            let phase = rfc.rfc.phase.as_ref();

            Row::new(vec![
                Line::from(rfc.rfc.rfc_id.clone()),
                Line::from(rfc.rfc.title.clone()),
                StatusText::new(status).render(),
                PhaseCell::new(phase).render(),
                TagsCell::new(&rfc.rfc.tags).render(),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

pub(super) fn draw_adr(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.index.adrs,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(10),
                Constraint::Min(40),
                Constraint::Length(14),
                Constraint::Min(15),
            ],
            compact_widths: vec![
                Constraint::Length(10),
                Constraint::Min(20),
                Constraint::Length(12),
                Constraint::Length(0),
            ],
            headers: &["ID", "Title", "Status", "Tags"],
            header_color: Color::Green,
            title: "ADR INDEX",
            border_color: Color::Green,
        },
        |adr| {
            let meta = adr.meta();
            ResourceListRow {
                id: &meta.id,
                title: &meta.title,
                status: meta.status.as_ref(),
                tags: &meta.tags,
            }
            .render()
        },
    )
    .render(frame, area, &mut app.table_state);
}

pub(super) fn draw_work(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.index.work_items,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(22),
                Constraint::Min(35),
                Constraint::Length(14),
                Constraint::Min(15),
            ],
            compact_widths: vec![
                Constraint::Length(22),
                Constraint::Min(14),
                Constraint::Length(12),
                Constraint::Length(0),
            ],
            headers: &["ID", "Title", "Status", "Tags"],
            header_color: Color::Yellow,
            title: "WORK QUEUE",
            border_color: Color::Yellow,
        },
        |item| {
            let meta = item.meta();
            ResourceListRow {
                id: &meta.id,
                title: &meta.title,
                status: meta.status.as_ref(),
                tags: &meta.tags,
            }
            .render()
        },
    )
    .render(frame, area, &mut app.table_state);
}

// Implements [[RFC-0007:C-COCKPIT-VIEWS]]: clause browsing view.
pub(super) fn draw_clause(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.supplement.clauses,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(10),
                Constraint::Length(18),
                Constraint::Min(30),
                Constraint::Length(12),
                Constraint::Min(12),
            ],
            compact_widths: vec![
                Constraint::Length(10),
                Constraint::Length(14),
                Constraint::Min(12),
                Constraint::Length(11),
                Constraint::Length(0),
            ],
            headers: &["RFC", "Clause", "Title", "Status", "Tags"],
            header_color: Color::Magenta,
            title: "CLAUSE INDEX",
            border_color: Color::Magenta,
        },
        |entry| {
            let clause = &entry.clause.spec;
            Row::new(vec![
                Line::from(entry.rfc_id.clone()),
                Line::from(clause.clause_id.clone()),
                Line::from(clause.title.clone()),
                StatusText::new(clause.status.as_ref()).render(),
                TagsCell::new(&clause.tags).render(),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

// Implements [[RFC-0007:C-COCKPIT-VIEWS]]: guard browsing view.
pub(super) fn draw_guard(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.supplement.guards,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(18),
                Constraint::Min(30),
                Constraint::Length(10),
                Constraint::Min(24),
            ],
            compact_widths: vec![
                Constraint::Length(18),
                Constraint::Min(14),
                Constraint::Length(8),
                Constraint::Length(0),
            ],
            headers: &["ID", "Title", "Timeout", "Command"],
            header_color: Color::LightBlue,
            title: "GUARD MATRIX",
            border_color: Color::LightBlue,
        },
        |guard| {
            Row::new(vec![
                Line::from(guard.meta().id.clone()),
                Line::from(guard.meta().title.clone()),
                Line::from(format!("{}s", guard.spec.check.timeout_secs)),
                Line::from(guard.spec.check.command.clone()),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

// Implements [[RFC-0007:C-CONFORMANCE-VIEWS]]: declared trace relationships are browsable.
pub(super) fn draw_conformance(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let cases = app
        .index
        .conformance_cases
        .iter()
        .map(|case| crate::model::derive_conformance_trace(case, &app.index))
        .collect::<Vec<_>>();
    ResourceTable::from_indexed_items(
        &cases,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Min(26),
                Constraint::Min(16),
                Constraint::Length(13),
                Constraint::Min(20),
                Constraint::Min(14),
                Constraint::Min(13),
            ],
            compact_widths: vec![
                Constraint::Length(28),
                Constraint::Min(16),
                Constraint::Length(12),
                Constraint::Length(0),
                Constraint::Length(0),
                Constraint::Length(0),
            ],
            headers: &[
                "ID",
                "Title",
                "Applicability",
                "Scenario path",
                "Selector",
                "Guards",
            ],
            header_color: Color::LightMagenta,
            title: "CASE TRACE",
            border_color: Color::LightMagenta,
        },
        |case| {
            Row::new(vec![
                Line::from(case.id.clone()),
                Line::from(case.title.clone()),
                Line::styled(
                    case.requirement_applicability.to_string(),
                    applicability_style(case.requirement_applicability),
                ),
                Line::from(case.path.clone()),
                Line::from(case.selector.clone()),
                Line::from(case.guards.join(", ")),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

fn applicability_style(applicability: ConformanceApplicability) -> Style {
    let color = match applicability {
        ConformanceApplicability::Stale => Color::Red,
        ConformanceApplicability::Provisional => Color::Yellow,
        ConformanceApplicability::Candidate => Color::Cyan,
        ConformanceApplicability::Current => Color::Green,
    };
    Style::default().fg(color).bold()
}

// Implements [[RFC-0007:C-COCKPIT-VIEWS]]: release browsing view.
pub(super) fn draw_release(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.supplement.releases,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Min(30),
            ],
            compact_widths: vec![
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(6),
                Constraint::Min(20),
            ],
            headers: &["Version", "Date", "Refs", "Work Items"],
            header_color: Color::Cyan,
            title: "RELEASE LOG",
            border_color: Color::Cyan,
        },
        |release| {
            Row::new(vec![
                Line::from(release.version.clone()),
                Line::from(release.date.clone()),
                Line::from(release.refs.len().to_string()),
                Line::from(release.refs.join(", ")),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

// Implements [[RFC-0007:C-COCKPIT-VIEWS]]: tag browsing view.
pub(super) fn draw_tag(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.supplement.tags,
        &indices,
        ResourceTableSpec {
            widths: vec![Constraint::Min(20), Constraint::Length(8)],
            compact_widths: vec![Constraint::Min(20), Constraint::Length(8)],
            headers: &["Tag", "Count"],
            header_color: Color::Magenta,
            title: "TAG INDEX",
            border_color: Color::Magenta,
        },
        |tag| {
            Row::new(vec![
                Line::from(tag.name.clone()),
                Line::from(tag.count.to_string()),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

// Implements [[RFC-0007:C-LOOP-VIEWS]]: loop list view.
pub(super) fn draw_loop(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        app.loop_entries(),
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(22),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(16),
                Constraint::Min(28),
            ],
            compact_widths: vec![
                Constraint::Length(20),
                Constraint::Length(10),
                Constraint::Length(6),
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Length(0),
            ],
            headers: &["ID", "State", "Items", "Rounds", "Action", "Work"],
            header_color: Color::Yellow,
            title: "LOOP CONTROL",
            border_color: Color::Yellow,
        },
        |entry| {
            if let Some(state) = &entry.state {
                Row::new(vec![
                    Line::from(entry.id.clone()),
                    Line::from(state.loop_meta.state.as_str()),
                    Line::from(state.loop_meta.resolved.len().to_string()),
                    Line::from(
                        state
                            .items
                            .values()
                            .map(|item| item.round_count)
                            .sum::<u32>()
                            .to_string(),
                    ),
                    Line::from(state.loop_meta.next_action.as_str()),
                    Line::from(state.loop_meta.work.join(", ")),
                ])
            } else {
                Row::new(vec![
                    Line::from(entry.id.clone()),
                    Line::from("invalid"),
                    Line::from("-"),
                    Line::from("-"),
                    Line::from("-"),
                    Line::from(
                        entry
                            .diagnostic
                            .as_ref()
                            .map(|diag| diag.message.clone())
                            .unwrap_or_default(),
                    ),
                ])
            }
        },
    )
    .render(frame, area, &mut app.table_state);
}

// Implements [[RFC-0007:C-SEARCH]]: search results view.
pub(super) fn draw_search(frame: &mut Frame, app: &mut App, area: Rect) {
    if let Some(diagnostic) = &app.search_error {
        let message = Text::from(vec![
            Line::from(vec![
                Span::styled("Search failed: ", Style::default().fg(Color::Red).bold()),
                Span::raw(diagnostic.message.clone()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Code:   ", Style::default().fg(Color::DarkGray)),
                Span::raw(diagnostic.code.code()),
            ]),
            Line::from(vec![
                Span::styled("Target: ", Style::default().fg(Color::DarkGray)),
                Span::raw(diagnostic.file.clone()),
            ]),
        ]);
        frame.render_widget(
            Paragraph::new(message)
                .wrap(Wrap { trim: false })
                .block(panel_block("SEARCH").border_style(Style::default().fg(Color::Red))),
            area,
        );
        return;
    }

    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.search_results,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(8),
                Constraint::Length(18),
                Constraint::Min(28),
                Constraint::Min(36),
            ],
            compact_widths: vec![
                Constraint::Length(7),
                Constraint::Length(16),
                Constraint::Min(20),
                Constraint::Length(0),
            ],
            headers: &["Kind", "ID", "Title", "Snippet"],
            header_color: Color::Green,
            title: "SEARCH",
            border_color: Color::Green,
        },
        |result| {
            Row::new(vec![
                Line::from(result.kind.clone()),
                Line::from(result.id.clone()),
                Line::from(result.title.clone()),
                Line::from(result.snippet.clone()),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

// Implements [[RFC-0007:C-DIAGNOSTICS]]: check diagnostics view.
pub(super) fn draw_diagnostics(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    ResourceTable::from_indexed_items(
        &app.supplement.diagnostics,
        &indices,
        ResourceTableSpec {
            widths: vec![
                Constraint::Length(9),
                Constraint::Length(8),
                Constraint::Min(44),
                Constraint::Min(24),
            ],
            compact_widths: vec![
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Min(28),
                Constraint::Length(0),
            ],
            headers: &["Level", "Code", "Message", "Target"],
            header_color: Color::Red,
            title: "DIAGNOSTICS",
            border_color: Color::Red,
        },
        |diagnostic| {
            Row::new(vec![
                Line::from(level_label(diagnostic.level)),
                Line::from(diagnostic.code.code()),
                Line::from(diagnostic.message.clone()),
                Line::from(diagnostic.file.clone()),
            ])
        },
    )
    .render(frame, area, &mut app.table_state);
}

fn level_label(level: DiagnosticLevel) -> &'static str {
    match level {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info => "info",
    }
}

#[cfg(test)]
#[path = "lists_tests.rs"]
mod tests;
