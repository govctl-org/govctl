use super::super::app::App;
use super::components::{
    PhaseCell, ResourceListRow, ResourceTable, ResourceTableSpec, StatusText, TagsCell,
};
use ratatui::{prelude::*, widgets::Row};

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

#[cfg(test)]
#[path = "lists_tests.rs"]
mod tests;
