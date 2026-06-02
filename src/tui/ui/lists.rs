use super::super::app::App;
use super::{phase_style, rounded_block, status_style};
use crate::theme::status_icon;
use ratatui::{
    prelude::*,
    widgets::{Row, Table},
};

pub(super) fn draw_rfc(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows = indices
        .iter()
        .filter_map(|&idx| app.index.rfcs.get(idx))
        .map(|rfc| {
            let status = rfc.rfc.status.as_ref();
            let phase = rfc.rfc.phase.as_ref();

            Row::new(vec![
                Line::from(rfc.rfc.rfc_id.clone()),
                Line::from(rfc.rfc.title.clone()),
                status_cell(status),
                Line::from(Span::styled(phase.to_string(), phase_style(phase))),
                tags_cell(&rfc.rfc.tags),
            ])
        })
        .collect::<Vec<_>>();

    render_table(
        frame,
        app,
        area,
        rows,
        TableSpec {
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
    );
}

pub(super) fn draw_adr(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows = indices
        .iter()
        .filter_map(|&idx| app.index.adrs.get(idx))
        .map(|adr| {
            let meta = adr.meta();
            ResourceListRow {
                id: &meta.id,
                title: &meta.title,
                status: meta.status.as_ref(),
                tags: &meta.tags,
            }
            .render()
        })
        .collect::<Vec<_>>();

    render_table(
        frame,
        app,
        area,
        rows,
        TableSpec {
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
    );
}

pub(super) fn draw_work(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows = indices
        .iter()
        .filter_map(|&idx| app.index.work_items.get(idx))
        .map(|item| {
            let meta = item.meta();
            ResourceListRow {
                id: &meta.id,
                title: &meta.title,
                status: meta.status.as_ref(),
                tags: &meta.tags,
            }
            .render()
        })
        .collect::<Vec<_>>();

    render_table(
        frame,
        app,
        area,
        rows,
        TableSpec {
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
    );
}

struct ResourceListRow<'a> {
    id: &'a str,
    title: &'a str,
    status: &'a str,
    tags: &'a [String],
}

impl ResourceListRow<'_> {
    fn render(&self) -> Row<'static> {
        Row::new(vec![
            Line::from(self.id.to_string()),
            Line::from(self.title.to_string()),
            status_cell(self.status),
            tags_cell(self.tags),
        ])
    }
}

fn status_cell(status: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{} ", status_icon(status)), status_style(status)),
        Span::styled(status.to_string(), status_style(status)),
    ])
}

fn tags_cell(tags: &[String]) -> Line<'static> {
    Line::from(Span::styled(
        tags.join(" "),
        Style::default().fg(Color::Magenta),
    ))
}

struct TableSpec {
    widths: Vec<Constraint>,
    headers: &'static [&'static str],
    header_color: Color,
    title: &'static str,
    border_color: Color,
}

fn render_table(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    rows: Vec<Row<'static>>,
    spec: TableSpec,
) {
    let table = Table::new(rows, spec.widths)
        .header(
            Row::new(spec.headers.to_vec())
                .style(Style::default().bold().fg(spec.header_color))
                .bottom_margin(1),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .block(rounded_block(spec.title).border_style(Style::default().fg(spec.border_color)));
    frame.render_stateful_widget(table, area, &mut app.table_state);
}

#[cfg(test)]
#[path = "lists_tests.rs"]
mod tests;
