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
            let status = meta.status.as_ref();

            Row::new(vec![
                Line::from(meta.id.clone()),
                Line::from(meta.title.clone()),
                status_cell(status),
                tags_cell(&meta.tags),
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
            let status = meta.status.as_ref();

            Row::new(vec![
                Line::from(meta.id.clone()),
                Line::from(meta.title.clone()),
                status_cell(status),
                tags_cell(&meta.tags),
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
mod tests {
    use super::super::super::app::View;
    use super::super::test_support::{adr, buffer_lines, project_index, rfc, work_item};
    use super::*;
    use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn list_renderers_draw_table_rows() -> Result<(), Box<dyn std::error::Error>> {
        let rendered = render_list(View::RfcList, draw_rfc)?;
        assert!(rendered.iter().any(|line| line.contains("RFC-0001")));
        assert!(rendered.iter().any(|line| line.contains("RFC title")));
        assert!(rendered.iter().any(|line| line.contains("normative")));

        let rendered = render_list(View::AdrList, draw_adr)?;
        assert!(rendered.iter().any(|line| line.contains("ADR-0001")));
        assert!(rendered.iter().any(|line| line.contains("ADR title")));
        assert!(rendered.iter().any(|line| line.contains("accepted")));

        let rendered = render_list(View::WorkList, draw_work)?;
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("WI-2026-01-01-001"))
        );
        assert!(rendered.iter().any(|line| line.contains("Work title")));
        assert!(rendered.iter().any(|line| line.contains("active")));
        Ok(())
    }

    fn render_list(
        view: View,
        draw: fn(&mut Frame, &mut App, Rect),
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let backend = TestBackend::new(110, 8);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new(list_project_index());
        app.view = view;

        terminal.draw(|frame| draw(frame, &mut app, frame.area()))?;

        Ok(buffer_lines(terminal.backend().buffer()))
    }

    fn list_project_index() -> crate::model::ProjectIndex {
        project_index(
            vec![rfc(
                "RFC-0001",
                "RFC title",
                RfcStatus::Normative,
                RfcPhase::Impl,
                &["core"],
            )],
            vec![adr(
                "ADR-0001",
                "ADR title",
                AdrStatus::Accepted,
                &["design"],
            )],
            vec![work_item(
                "WI-2026-01-01-001",
                "Work title",
                WorkItemStatus::Active,
                &["cleanup"],
            )],
        )
    }
}
