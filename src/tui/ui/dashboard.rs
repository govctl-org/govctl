use super::super::app::App;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Borders, Paragraph},
};

pub(super) fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    frame.render_widget(rfc_stats(app), content_chunks[0]);
    frame.render_widget(adr_stats(app), content_chunks[1]);
    frame.render_widget(work_stats(app), content_chunks[2]);
}

fn summary_block(
    title: &str,
    border_color: Color,
    mut lines: Vec<Line<'static>>,
) -> Paragraph<'static> {
    lines.insert(0, Line::from(""));
    Paragraph::new(lines).block(
        Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(border_color)),
    )
}

fn summary_line(icon: &'static str, icon_color: Color, label: &str, value: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(icon, Style::default().fg(icon_color)),
        Span::raw(format!(" {}{}", label, value)),
    ])
}

fn total_line(total: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled("Σ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(format!(" Total: {}", total), Style::default().bold()),
    ])
}

fn rfc_stats(app: &App) -> Paragraph<'static> {
    let mut draft = 0;
    let mut normative = 0;
    let mut deprecated = 0;

    for rfc in &app.index.rfcs {
        match rfc.rfc.status.as_ref() {
            "draft" => draft += 1,
            "normative" => normative += 1,
            "deprecated" => deprecated += 1,
            _ => {}
        }
    }

    summary_block(
        "📋 RFCs",
        Color::Blue,
        vec![
            summary_line("○", Color::Yellow, "Draft:      ", draft),
            summary_line("●", Color::Green, "Normative:  ", normative),
            summary_line("✗", Color::Red, "Deprecated: ", deprecated),
            Line::from(""),
            total_line(app.index.rfcs.len()),
        ],
    )
}

fn adr_stats(app: &App) -> Paragraph<'static> {
    let mut proposed = 0;
    let mut accepted = 0;
    let mut superseded = 0;

    for adr in &app.index.adrs {
        match adr.meta().status.as_ref() {
            "proposed" => proposed += 1,
            "accepted" => accepted += 1,
            "superseded" => superseded += 1,
            _ => {}
        }
    }

    summary_block(
        "📝 ADRs",
        Color::Green,
        vec![
            summary_line("○", Color::Yellow, "Proposed:   ", proposed),
            summary_line("●", Color::Green, "Accepted:   ", accepted),
            summary_line("✗", Color::Red, "Superseded: ", superseded),
            Line::from(""),
            total_line(app.index.adrs.len()),
        ],
    )
}

fn work_stats(app: &App) -> Paragraph<'static> {
    let mut queue = 0;
    let mut active = 0;
    let mut done = 0;

    for item in &app.index.work_items {
        match item.meta().status.as_ref() {
            "queue" => queue += 1,
            "active" => active += 1,
            "done" => done += 1,
            _ => {}
        }
    }

    summary_block(
        "📌 Work Items",
        Color::Yellow,
        vec![
            summary_line("○", Color::Yellow, "Queue:  ", queue),
            summary_line("◉", Color::Green, "Active: ", active),
            summary_line("●", Color::Green, "Done:   ", done),
            Line::from(""),
            total_line(app.index.work_items.len()),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AdrContent, AdrEntry, AdrMeta, AdrSpec, AdrStatus, ProjectIndex, RfcIndex, RfcPhase,
        RfcSpec, RfcStatus, WorkItemContent, WorkItemEntry, WorkItemMeta, WorkItemSpec,
        WorkItemStatus, WorkItemVerification,
    };
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
    use std::path::PathBuf;

    #[test]
    fn dashboard_draws_summary_counts() -> Result<(), Box<dyn std::error::Error>> {
        let backend = TestBackend::new(90, 8);
        let mut terminal = Terminal::new(backend)?;
        let app = App::new(project_index());

        terminal.draw(|frame| draw(frame, &app, frame.area()))?;

        let rendered = buffer_lines(terminal.backend().buffer());
        assert!(rendered.iter().any(|line| line.contains("Draft:      1")));
        assert!(rendered.iter().any(|line| line.contains("Normative:  1")));
        assert!(rendered.iter().any(|line| line.contains("Proposed:   1")));
        assert!(rendered.iter().any(|line| line.contains("Accepted:   1")));
        assert!(rendered.iter().any(|line| line.contains("Queue:  1")));
        assert!(rendered.iter().any(|line| line.contains("Active: 1")));
        Ok(())
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
            rfcs: vec![
                rfc("RFC-0001", RfcStatus::Draft),
                rfc("RFC-0002", RfcStatus::Normative),
            ],
            adrs: vec![
                adr("ADR-0001", AdrStatus::Proposed),
                adr("ADR-0002", AdrStatus::Accepted),
            ],
            work_items: vec![
                work_item("WI-2026-01-01-001", WorkItemStatus::Queue),
                work_item("WI-2026-01-01-002", WorkItemStatus::Active),
            ],
        }
    }

    fn rfc(id: &str, status: RfcStatus) -> RfcIndex {
        RfcIndex {
            rfc: RfcSpec {
                rfc_id: id.to_string(),
                title: id.to_string(),
                version: "0.1.0".to_string(),
                status,
                phase: RfcPhase::Spec,
                owners: vec![],
                created: "2026-01-01".to_string(),
                updated: None,
                supersedes: None,
                refs: vec![],
                tags: vec![],
                sections: vec![],
                changelog: vec![],
                signature: None,
            },
            clauses: vec![],
            path: PathBuf::from(format!("gov/rfc/{id}.toml")),
        }
    }

    fn adr(id: &str, status: AdrStatus) -> AdrEntry {
        AdrEntry {
            spec: AdrSpec {
                govctl: AdrMeta {
                    schema: 1,
                    id: id.to_string(),
                    title: id.to_string(),
                    status,
                    date: "2026-01-01".to_string(),
                    superseded_by: None,
                    refs: vec![],
                    tags: vec![],
                },
                content: AdrContent::default(),
            },
            path: PathBuf::from(format!("gov/adr/{id}.toml")),
        }
    }

    fn work_item(id: &str, status: WorkItemStatus) -> WorkItemEntry {
        WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 1,
                    id: id.to_string(),
                    title: id.to_string(),
                    status,
                    created: None,
                    started: None,
                    completed: None,
                    refs: vec![],
                    depends_on: vec![],
                    tags: vec![],
                },
                content: WorkItemContent::default(),
                verification: WorkItemVerification::default(),
            },
            path: PathBuf::from(format!("gov/work/{id}.toml")),
        }
    }
}
