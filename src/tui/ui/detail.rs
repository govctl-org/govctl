use super::super::app::App;
use super::components::{DetailViewport, MarkdownPanel, MetadataLine};
use super::{rounded_block, status_style};
use crate::theme::status_icon;
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph},
};

pub(super) fn draw_rfc(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) {
    let Some(rfc) = app.index.rfcs.get(idx) else {
        return;
    };

    let status = rfc.rfc.status.as_ref();
    let phase = rfc.rfc.phase.as_ref();

    let mut header_lines = vec![
        MetadataLine::styled("ID:      ", &rfc.rfc.rfc_id, Style::default().bold()).render(),
        MetadataLine::plain("Title:   ", &rfc.rfc.title).render(),
        MetadataLine::styled(
            "Version: ",
            &rfc.rfc.version,
            Style::default().fg(Color::Cyan),
        )
        .render(),
        MetadataLine::status("Status:  ", status).render(),
        MetadataLine::phase("Phase:   ", phase).render(),
        MetadataLine::joined("Owners:  ", &rfc.rfc.owners, ", ").render(),
    ];

    if !rfc.rfc.refs.is_empty() {
        header_lines.push(MetadataLine::joined("Refs:    ", &rfc.rfc.refs, ", ").render());
    }

    if !rfc.rfc.tags.is_empty() {
        header_lines.push(MetadataLine::tags("Tags:    ", &rfc.rfc.tags).render());
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

pub(super) fn draw_adr(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> DetailViewport {
    let Some(adr) = app.index.adrs.get(idx) else {
        return DetailViewport::new(0);
    };

    let text = crate::render::render_adr(adr)
        .map(|md| crate::terminal_md::render_to_tui_text(&md))
        .unwrap_or_default();

    let title = format!("📝 {}", adr.meta().id);
    draw_markdown_panel(frame, area, app.scroll, &title, Color::Green, text)
}

pub(super) fn draw_work(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    idx: usize,
) -> DetailViewport {
    let Some(item) = app.index.work_items.get(idx) else {
        return DetailViewport::new(0);
    };

    let text = crate::render::render_work_item(item)
        .map(|md| crate::terminal_md::render_to_tui_text(&md))
        .unwrap_or_default();

    let title = format!("📌 {}", item.meta().id);
    draw_markdown_panel(frame, area, app.scroll, &title, Color::Yellow, text)
}

pub(super) fn draw_clause(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    rfc_idx: usize,
    clause_idx: usize,
) -> DetailViewport {
    let Some(rfc) = app.index.rfcs.get(rfc_idx) else {
        return DetailViewport::new(0);
    };

    let Some(clause) = rfc.clauses.get(clause_idx) else {
        return DetailViewport::new(0);
    };

    let mut raw = String::new();
    crate::render::render_clause(&mut raw, &rfc.rfc.rfc_id, clause);
    let text = crate::terminal_md::render_to_tui_text(&raw);

    let title = format!("📜 {}", clause.spec.clause_id);
    draw_markdown_panel(frame, area, app.scroll, &title, Color::Magenta, text)
}

fn draw_markdown_panel(
    frame: &mut Frame,
    area: Rect,
    scroll: u16,
    title: &str,
    border_color: Color,
    text: Text<'_>,
) -> DetailViewport {
    MarkdownPanel::new(title, border_color, scroll, text).render(frame, area)
}

#[cfg(test)]
#[path = "detail_tests.rs"]
mod tests;
