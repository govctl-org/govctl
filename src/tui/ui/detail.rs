use super::super::app::App;
use super::{
    components::{ClauseListRow, SelectableList, StatusText},
    phase_style, rounded_block, wrapped_line_count,
};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use std::borrow::Cow;

struct MetadataPanel<'a> {
    title: String,
    border_color: Color,
    lines: Vec<Line<'a>>,
}

impl<'a> MetadataPanel<'a> {
    fn new(title: impl Into<String>, border_color: Color, lines: Vec<Line<'a>>) -> Self {
        Self {
            title: title.into(),
            border_color,
            lines,
        }
    }

    fn outer_height(&self) -> u16 {
        (self.lines.len() as u16) + 2
    }

    fn render(self, frame: &mut Frame, area: Rect) {
        let block = rounded_block(&self.title).border_style(Style::default().fg(self.border_color));
        let panel = Paragraph::new(self.lines).block(block);
        frame.render_widget(panel, area);
    }
}

struct MarkdownPanel<'a> {
    title: &'a str,
    border_color: Color,
    scroll: u16,
    text: Text<'a>,
}

impl<'a> MarkdownPanel<'a> {
    fn new(title: &'a str, border_color: Color, scroll: u16, text: Text<'a>) -> Self {
        Self {
            title,
            border_color,
            scroll,
            text,
        }
    }

    fn render(self, frame: &mut Frame, area: Rect) -> DetailViewport {
        let block = rounded_block(self.title).border_style(Style::default().fg(self.border_color));
        let inner_width = block.inner(area).width;
        let total_lines = wrapped_line_count(&self.text.lines, inner_width);
        let content = Paragraph::new(self.text)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0))
            .block(block);

        frame.render_widget(content, area);
        DetailViewport::new(total_lines)
    }
}

struct MarkdownDetailPanel<'a> {
    title: &'a str,
    border_color: Color,
    scroll: u16,
    markdown: &'a str,
}

impl<'a> MarkdownDetailPanel<'a> {
    fn new(title: &'a str, border_color: Color, scroll: u16, markdown: &'a str) -> Self {
        Self {
            title,
            border_color,
            scroll,
            markdown,
        }
    }

    fn render(self, frame: &mut Frame, area: Rect) -> DetailViewport {
        let text = crate::terminal_md::render_to_tui_text(self.markdown);
        MarkdownPanel::new(self.title, self.border_color, self.scroll, text).render(frame, area)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DetailViewport {
    total_lines: usize,
}

impl DetailViewport {
    fn new(total_lines: usize) -> Self {
        Self { total_lines }
    }

    pub(super) fn footer_status(self, scroll: &mut u16) -> String {
        let max_scroll = self.total_lines.saturating_sub(1) as u16;
        if *scroll > max_scroll {
            *scroll = max_scroll;
        }
        format!(
            "Scroll {}/{}",
            (*scroll).saturating_add(1),
            self.total_lines
        )
    }
}

struct MetadataLine<'a> {
    label: &'static str,
    value: Vec<Span<'a>>,
}

impl<'a> MetadataLine<'a> {
    fn plain(label: &'static str, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label,
            value: vec![Span::raw(value.into())],
        }
    }

    fn styled(label: &'static str, value: impl Into<Cow<'a, str>>, style: Style) -> Self {
        Self {
            label,
            value: vec![Span::styled(value.into(), style)],
        }
    }

    fn status(label: &'static str, status: &str) -> Self {
        Self {
            label,
            value: StatusText::new(status).spans(),
        }
    }

    fn phase(label: &'static str, phase: &str) -> Self {
        Self {
            label,
            value: vec![Span::styled(phase.to_string(), phase_style(phase))],
        }
    }

    fn joined(label: &'static str, values: &[String], separator: &str) -> Self {
        Self::plain(label, values.join(separator))
    }

    fn tags(label: &'static str, tags: &[String]) -> Self {
        Self {
            label,
            value: vec![Span::styled(
                tags.join("  "),
                Style::default().fg(Color::Magenta).bold(),
            )],
        }
    }

    fn render(mut self) -> Line<'a> {
        let mut spans = vec![Span::styled(
            self.label,
            Style::default().fg(Color::DarkGray),
        )];
        spans.append(&mut self.value);
        Line::from(spans)
    }
}

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

    let header_panel =
        MetadataPanel::new(format!("📋 {}", rfc.rfc.rfc_id), Color::Blue, header_lines);
    let header_height = header_panel.outer_height();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(5)])
        .split(area);

    header_panel.render(frame, chunks[0]);

    let clause_items = rfc
        .clauses
        .iter()
        .map(|clause| {
            let clause_status = clause.spec.status.as_ref();
            ClauseListRow {
                id: &clause.spec.clause_id,
                title: &clause.spec.title,
                status: clause_status,
            }
            .render()
        })
        .collect();

    SelectableList::new("Clauses", Color::Cyan, clause_items).render(
        frame,
        chunks[1],
        &mut app.clause_list_state,
    );
}

pub(super) fn draw_adr(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> DetailViewport {
    let Some(adr) = app.index.adrs.get(idx) else {
        return DetailViewport::new(0);
    };

    let markdown = crate::render::render_adr(adr).unwrap_or_default();
    let title = format!("📝 {}", adr.meta().id);
    MarkdownDetailPanel::new(&title, Color::Green, app.scroll, &markdown).render(frame, area)
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

    let markdown = crate::render::render_work_item(item).unwrap_or_default();
    let title = format!("📌 {}", item.meta().id);
    MarkdownDetailPanel::new(&title, Color::Yellow, app.scroll, &markdown).render(frame, area)
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

    let title = format!("📜 {}", clause.spec.clause_id);
    MarkdownDetailPanel::new(&title, Color::Magenta, app.scroll, &raw).render(frame, area)
}

#[cfg(test)]
#[path = "detail_tests.rs"]
mod tests;
