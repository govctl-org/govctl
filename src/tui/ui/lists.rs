use super::super::app::App;
use super::components::{
    PhaseCell, ResourceListRow, ResourceTable, ResourceTableSpec, StatusText, TagsCell,
};
use super::rounded_block;
use crate::diagnostic::DiagnosticLevel;
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
            headers: &["ID", "Title", "Status", "Phase", "Tags"],
            header_color: Color::Cyan,
            title: "📋 RFCs",
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
            headers: &["ID", "Title", "Status", "Tags"],
            header_color: Color::Green,
            title: "📝 ADRs",
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
            headers: &["ID", "Title", "Status", "Tags"],
            header_color: Color::Yellow,
            title: "📌 Work Items",
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
            headers: &["RFC", "Clause", "Title", "Status", "Tags"],
            header_color: Color::Magenta,
            title: "Clauses",
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
            headers: &["ID", "Title", "Timeout", "Command"],
            header_color: Color::LightBlue,
            title: "Guards",
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
            headers: &["Version", "Date", "Refs", "Work Items"],
            header_color: Color::Cyan,
            title: "Releases",
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
            headers: &["Tag", "Count"],
            header_color: Color::Magenta,
            title: "Tags",
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
            headers: &["ID", "State", "Items", "Rounds", "Action", "Work"],
            header_color: Color::Yellow,
            title: "Loops",
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
                .block(rounded_block("Search").border_style(Style::default().fg(Color::Red))),
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
            headers: &["Kind", "ID", "Title", "Snippet"],
            header_color: Color::Green,
            title: "Search",
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
            headers: &["Level", "Code", "Message", "Target"],
            header_color: Color::Red,
            title: "Diagnostics",
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
