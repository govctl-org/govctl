use super::super::app::App;
use super::{phase_style, rounded_block, status_style};
use crate::theme::status_icon;
use ratatui::{
    prelude::*,
    widgets::{Row, Table},
};

pub(super) fn draw_rfc(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows: Vec<Row> = indices
        .iter()
        .filter_map(|&idx| app.index.rfcs.get(idx))
        .map(|rfc| {
            let status = rfc.rfc.status.as_ref();
            let phase = rfc.rfc.phase.as_ref();
            let tags = rfc.rfc.tags.join(" ");

            Row::new(vec![
                Line::from(rfc.rfc.rfc_id.clone()),
                Line::from(rfc.rfc.title.clone()),
                Line::from(vec![
                    Span::styled(format!("{} ", status_icon(status)), status_style(status)),
                    Span::styled(status.to_string(), status_style(status)),
                ]),
                Line::from(Span::styled(phase.to_string(), phase_style(phase))),
                Line::from(Span::styled(tags, Style::default().fg(Color::Magenta))),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Min(20),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Min(15),
        ],
    )
    .header(
        Row::new(vec!["ID", "Title", "Status", "Phase", "Tags"])
            .style(Style::default().bold().fg(Color::Cyan))
            .bottom_margin(1),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .block(rounded_block("📋 RFCs").border_style(Style::default().fg(Color::Blue)));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

pub(super) fn draw_adr(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows: Vec<Row> = indices
        .iter()
        .filter_map(|&idx| app.index.adrs.get(idx))
        .map(|adr| {
            let meta = adr.meta();
            let status = meta.status.as_ref();
            let tags = meta.tags.join(" ");

            Row::new(vec![
                Line::from(meta.id.clone()),
                Line::from(meta.title.clone()),
                Line::from(vec![
                    Span::styled(format!("{} ", status_icon(status)), status_style(status)),
                    Span::styled(status.to_string(), status_style(status)),
                ]),
                Line::from(Span::styled(tags, Style::default().fg(Color::Magenta))),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Min(40),
            Constraint::Length(14),
            Constraint::Min(15),
        ],
    )
    .header(
        Row::new(vec!["ID", "Title", "Status", "Tags"])
            .style(Style::default().bold().fg(Color::Green))
            .bottom_margin(1),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .block(rounded_block("📝 ADRs").border_style(Style::default().fg(Color::Green)));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

pub(super) fn draw_work(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows: Vec<Row> = indices
        .iter()
        .filter_map(|&idx| app.index.work_items.get(idx))
        .map(|item| {
            let meta = item.meta();
            let status = meta.status.as_ref();
            let tags = meta.tags.join(" ");

            Row::new(vec![
                Line::from(meta.id.clone()),
                Line::from(meta.title.clone()),
                Line::from(vec![
                    Span::styled(format!("{} ", status_icon(status)), status_style(status)),
                    Span::styled(status.to_string(), status_style(status)),
                ]),
                Line::from(Span::styled(tags, Style::default().fg(Color::Magenta))),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(22),
            Constraint::Min(35),
            Constraint::Length(14),
            Constraint::Min(15),
        ],
    )
    .header(
        Row::new(vec!["ID", "Title", "Status", "Tags"])
            .style(Style::default().bold().fg(Color::Yellow))
            .bottom_margin(1),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .block(rounded_block("📌 Work Items").border_style(Style::default().fg(Color::Yellow)));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

#[cfg(test)]
mod tests {
    use super::super::super::app::View;
    use super::*;
    use crate::model::{
        AdrContent, AdrEntry, AdrMeta, AdrSpec, AdrStatus, ProjectIndex, RfcIndex, RfcPhase,
        RfcSpec, RfcStatus, WorkItemContent, WorkItemEntry, WorkItemMeta, WorkItemSpec,
        WorkItemStatus, WorkItemVerification,
    };
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
    use std::path::PathBuf;

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
        let mut app = App::new(project_index());
        app.view = view;

        terminal.draw(|frame| draw(frame, &mut app, frame.area()))?;

        Ok(buffer_lines(terminal.backend().buffer()))
    }

    fn buffer_lines(buffer: &Buffer) -> Vec<String> {
        let width = buffer.area().width as usize;
        buffer
            .content()
            .chunks(width)
            .map(|row| {
                let mut line = String::new();
                for cell in row {
                    line.push_str(cell.symbol());
                }
                line
            })
            .collect()
    }

    fn project_index() -> ProjectIndex {
        ProjectIndex {
            rfcs: vec![rfc()],
            adrs: vec![adr()],
            work_items: vec![work_item()],
        }
    }

    fn rfc() -> RfcIndex {
        RfcIndex {
            rfc: RfcSpec {
                rfc_id: "RFC-0001".to_string(),
                title: "RFC title".to_string(),
                version: "0.1.0".to_string(),
                status: RfcStatus::Normative,
                phase: RfcPhase::Impl,
                owners: vec![],
                created: "2026-01-01".to_string(),
                updated: None,
                supersedes: None,
                refs: vec![],
                tags: vec!["core".to_string()],
                sections: vec![],
                changelog: vec![],
                signature: None,
            },
            clauses: vec![],
            path: PathBuf::from("gov/rfc/RFC-0001.toml"),
        }
    }

    fn adr() -> AdrEntry {
        AdrEntry {
            spec: AdrSpec {
                govctl: AdrMeta {
                    schema: 1,
                    id: "ADR-0001".to_string(),
                    title: "ADR title".to_string(),
                    status: AdrStatus::Accepted,
                    date: "2026-01-01".to_string(),
                    superseded_by: None,
                    refs: vec![],
                    tags: vec!["design".to_string()],
                },
                content: AdrContent::default(),
            },
            path: PathBuf::from("gov/adr/ADR-0001.toml"),
        }
    }

    fn work_item() -> WorkItemEntry {
        WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 1,
                    id: "WI-2026-01-01-001".to_string(),
                    title: "Work title".to_string(),
                    status: WorkItemStatus::Active,
                    created: None,
                    started: None,
                    completed: None,
                    refs: vec![],
                    depends_on: vec![],
                    tags: vec!["cleanup".to_string()],
                },
                content: WorkItemContent::default(),
                verification: WorkItemVerification::default(),
            },
            path: PathBuf::from("gov/work/WI-2026-01-01-001.toml"),
        }
    }
}
