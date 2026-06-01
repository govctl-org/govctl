use super::super::app::App;
use super::{phase_style, rounded_block, status_style, wrapped_line_count};
use crate::theme::status_icon;
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph, Wrap},
};

pub(super) fn draw_rfc(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) {
    let Some(rfc) = app.index.rfcs.get(idx) else {
        return;
    };

    let status = rfc.rfc.status.as_ref();
    let phase = rfc.rfc.phase.as_ref();

    let mut header_lines = vec![
        Line::from(vec![
            Span::styled("ID:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(rfc.rfc.rfc_id.clone(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Title:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(rfc.rfc.title.clone()),
        ]),
        Line::from(vec![
            Span::styled("Version: ", Style::default().fg(Color::DarkGray)),
            Span::styled(rfc.rfc.version.clone(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Status:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", status_icon(status)), status_style(status)),
            Span::styled(status.to_string(), status_style(status)),
        ]),
        Line::from(vec![
            Span::styled("Phase:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(phase.to_string(), phase_style(phase)),
        ]),
        Line::from(vec![
            Span::styled("Owners:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(rfc.rfc.owners.join(", ")),
        ]),
    ];

    if !rfc.rfc.refs.is_empty() {
        header_lines.push(Line::from(vec![
            Span::styled("Refs:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(rfc.rfc.refs.join(", ")),
        ]));
    }

    if !rfc.rfc.tags.is_empty() {
        header_lines.push(Line::from(vec![
            Span::styled("Tags:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                rfc.rfc.tags.join("  "),
                Style::default().fg(Color::Magenta).bold(),
            ),
        ]));
    }

    let header_height = (header_lines.len() as u16) + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(5)])
        .split(area);

    let title = format!("📋 {}", rfc.rfc.rfc_id);
    let header = Paragraph::new(header_lines)
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Blue)));
    frame.render_widget(header, chunks[0]);

    let clause_items: Vec<ListItem> = rfc
        .clauses
        .iter()
        .map(|clause| {
            let clause_status = clause.spec.status.as_ref();
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon(clause_status)),
                    status_style(clause_status),
                ),
                Span::styled(
                    clause.spec.clause_id.clone(),
                    Style::default().fg(Color::Blue).bold(),
                ),
                Span::raw(" — "),
                Span::raw(clause.spec.title.clone()),
            ]))
        })
        .collect();

    let clause_list = List::new(clause_items)
        .block(rounded_block("Clauses").border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(clause_list, chunks[1], &mut app.clause_list_state);
}

pub(super) fn draw_adr(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> usize {
    let Some(adr) = app.index.adrs.get(idx) else {
        return 0;
    };

    let text = crate::render::render_adr(adr)
        .map(|md| crate::terminal_md::render_to_tui_text(&md))
        .unwrap_or_default();

    let title = format!("📝 {}", adr.meta().id);
    let block = rounded_block(&title).border_style(Style::default().fg(Color::Green));
    let inner_width = block.inner(area).width;
    let total_lines = wrapped_line_count(&text.lines, inner_width);
    let content = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(block);

    frame.render_widget(content, area);
    total_lines
}

pub(super) fn draw_work(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> usize {
    let Some(item) = app.index.work_items.get(idx) else {
        return 0;
    };

    let text = crate::render::render_work_item(item)
        .map(|md| crate::terminal_md::render_to_tui_text(&md))
        .unwrap_or_default();

    let title = format!("📌 {}", item.meta().id);
    let block = rounded_block(&title).border_style(Style::default().fg(Color::Yellow));
    let inner_width = block.inner(area).width;
    let total_lines = wrapped_line_count(&text.lines, inner_width);
    let content = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(block);

    frame.render_widget(content, area);
    total_lines
}

pub(super) fn draw_clause(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    rfc_idx: usize,
    clause_idx: usize,
) -> usize {
    let Some(rfc) = app.index.rfcs.get(rfc_idx) else {
        return 0;
    };

    let Some(clause) = rfc.clauses.get(clause_idx) else {
        return 0;
    };

    let mut raw = String::new();
    crate::render::render_clause(&mut raw, &rfc.rfc.rfc_id, clause);
    let text = crate::terminal_md::render_to_tui_text(&raw);

    let title = format!("📜 {}", clause.spec.clause_id);
    let block = rounded_block(&title).border_style(Style::default().fg(Color::Magenta));
    let inner_width = block.inner(area).width;
    let total_lines = wrapped_line_count(&text.lines, inner_width);
    let content = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(block);

    frame.render_widget(content, area);
    total_lines
}

#[cfg(test)]
mod tests {
    use super::super::super::app::{App, View};
    use super::super::test_support::{adr, buffer_lines, clause, project_index, rfc, work_item};
    use super::*;
    use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn detail_renderers_draw_expected_content() -> Result<(), Box<dyn std::error::Error>> {
        let rendered = render_detail(View::RfcDetail(0), |frame, app, area| {
            draw_rfc(frame, app, area, 0);
        })?;
        assert!(rendered.iter().any(|line| line.contains("RFC-0001")));
        assert!(rendered.iter().any(|line| line.contains("RFC title")));
        assert!(rendered.iter().any(|line| line.contains("C-TEST")));

        let rendered = render_detail(View::AdrDetail(0), |frame, app, area| {
            draw_adr(frame, app, area, 0);
        })?;
        assert!(rendered.iter().any(|line| line.contains("ADR-0001")));
        assert!(rendered.iter().any(|line| line.contains("ADR title")));

        let rendered = render_detail(View::WorkDetail(0), |frame, app, area| {
            draw_work(frame, app, area, 0);
        })?;
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("WI-2026-01-01-001"))
        );
        assert!(rendered.iter().any(|line| line.contains("Work title")));

        let rendered = render_clause_detail()?;
        assert!(rendered.iter().any(|line| line.contains("C-TEST")));
        assert!(rendered.iter().any(|line| line.contains("Clause text")));

        Ok(())
    }

    fn render_detail(
        view: View,
        mut draw: impl FnMut(&mut Frame, &mut App, Rect),
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let backend = TestBackend::new(110, 12);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new(detail_project_index());
        app.view = view;

        terminal.draw(|frame| {
            draw(frame, &mut app, frame.area());
        })?;

        Ok(buffer_lines(terminal.backend().buffer()))
    }

    fn render_clause_detail() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let backend = TestBackend::new(110, 12);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new(detail_project_index());
        app.view = View::ClauseDetail(0, 0);

        terminal.draw(|frame| {
            draw_clause(frame, &mut app, frame.area(), 0, 0);
        })?;

        Ok(buffer_lines(terminal.backend().buffer()))
    }

    fn detail_project_index() -> crate::model::ProjectIndex {
        let mut rfc = rfc(
            "RFC-0001",
            "RFC title",
            RfcStatus::Normative,
            RfcPhase::Impl,
            &["core"],
        );
        rfc.clauses
            .push(clause("C-TEST", "Clause title", "Clause text"));

        project_index(
            vec![rfc],
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
