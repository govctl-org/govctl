use super::{phase_style, rounded_block, status_style, wrapped_line_count};
use crate::theme::status_icon;
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap},
};
use std::borrow::Cow;

pub(super) struct ResourceTable {
    rows: Vec<Row<'static>>,
    spec: ResourceTableSpec,
}

pub(super) struct ResourceTableSpec {
    pub(super) widths: Vec<Constraint>,
    pub(super) headers: &'static [&'static str],
    pub(super) header_color: Color,
    pub(super) title: &'static str,
    pub(super) border_color: Color,
}

impl ResourceTable {
    pub(super) fn new(rows: Vec<Row<'static>>, spec: ResourceTableSpec) -> Self {
        Self { rows, spec }
    }

    pub(super) fn render(self, frame: &mut Frame, area: Rect, state: &mut TableState) {
        let table = Table::new(self.rows, self.spec.widths)
            .header(
                Row::new(self.spec.headers.to_vec())
                    .style(Style::default().bold().fg(self.spec.header_color))
                    .bottom_margin(1),
            )
            .row_highlight_style(Style::default().bg(Color::DarkGray))
            .block(
                rounded_block(self.spec.title)
                    .border_style(Style::default().fg(self.spec.border_color)),
            );
        frame.render_stateful_widget(table, area, state);
    }
}

pub(super) struct ResourceListRow<'a> {
    pub(super) id: &'a str,
    pub(super) title: &'a str,
    pub(super) status: &'a str,
    pub(super) tags: &'a [String],
}

impl ResourceListRow<'_> {
    pub(super) fn render(&self) -> Row<'static> {
        Row::new(vec![
            Line::from(self.id.to_string()),
            Line::from(self.title.to_string()),
            StatusCell::new(self.status).render(),
            TagsCell::new(self.tags).render(),
        ])
    }
}

pub(super) struct ClauseListRow<'a> {
    pub(super) id: &'a str,
    pub(super) title: &'a str,
    pub(super) status: &'a str,
}

impl ClauseListRow<'_> {
    pub(super) fn render(&self) -> ListItem<'static> {
        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{} ", status_icon(self.status)),
                status_style(self.status),
            ),
            Span::styled(self.id.to_string(), Style::default().fg(Color::Blue).bold()),
            Span::raw(" — "),
            Span::raw(self.title.to_string()),
        ]))
    }
}

pub(super) struct StatusCell<'a> {
    status: &'a str,
}

impl<'a> StatusCell<'a> {
    pub(super) fn new(status: &'a str) -> Self {
        Self { status }
    }

    pub(super) fn render(&self) -> Line<'static> {
        Line::from(vec![
            Span::styled(
                format!("{} ", status_icon(self.status)),
                status_style(self.status),
            ),
            Span::styled(self.status.to_string(), status_style(self.status)),
        ])
    }
}

pub(super) struct PhaseCell<'a> {
    phase: &'a str,
}

impl<'a> PhaseCell<'a> {
    pub(super) fn new(phase: &'a str) -> Self {
        Self { phase }
    }

    pub(super) fn render(&self) -> Line<'static> {
        Line::from(Span::styled(
            self.phase.to_string(),
            phase_style(self.phase),
        ))
    }
}

pub(super) struct TagsCell<'a> {
    tags: &'a [String],
}

impl<'a> TagsCell<'a> {
    pub(super) fn new(tags: &'a [String]) -> Self {
        Self { tags }
    }

    pub(super) fn render(&self) -> Line<'static> {
        Line::from(Span::styled(
            self.tags.join(" "),
            Style::default().fg(Color::Magenta),
        ))
    }
}

pub(super) struct MetadataPanel<'a> {
    title: String,
    border_color: Color,
    lines: Vec<Line<'a>>,
}

impl<'a> MetadataPanel<'a> {
    pub(super) fn new(title: impl Into<String>, border_color: Color, lines: Vec<Line<'a>>) -> Self {
        Self {
            title: title.into(),
            border_color,
            lines,
        }
    }

    pub(super) fn outer_height(&self) -> u16 {
        (self.lines.len() as u16) + 2
    }

    pub(super) fn render(self, frame: &mut Frame, area: Rect) {
        let block = rounded_block(&self.title).border_style(Style::default().fg(self.border_color));
        let panel = Paragraph::new(self.lines).block(block);
        frame.render_widget(panel, area);
    }
}

pub(super) struct SelectableList {
    title: String,
    border_color: Color,
    items: Vec<ListItem<'static>>,
}

impl SelectableList {
    pub(super) fn new(
        title: impl Into<String>,
        border_color: Color,
        items: Vec<ListItem<'static>>,
    ) -> Self {
        Self {
            title: title.into(),
            border_color,
            items,
        }
    }

    pub(super) fn render(self, frame: &mut Frame, area: Rect, state: &mut ListState) {
        let list = List::new(self.items)
            .block(rounded_block(&self.title).border_style(Style::default().fg(self.border_color)))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, state);
    }
}

pub(super) struct MarkdownPanel<'a> {
    title: &'a str,
    border_color: Color,
    scroll: u16,
    text: Text<'a>,
}

impl<'a> MarkdownPanel<'a> {
    pub(super) fn new(title: &'a str, border_color: Color, scroll: u16, text: Text<'a>) -> Self {
        Self {
            title,
            border_color,
            scroll,
            text,
        }
    }

    pub(super) fn render(self, frame: &mut Frame, area: Rect) -> DetailViewport {
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

pub(super) struct MarkdownDetailPanel<'a> {
    title: &'a str,
    border_color: Color,
    scroll: u16,
    markdown: &'a str,
}

impl<'a> MarkdownDetailPanel<'a> {
    pub(super) fn new(title: &'a str, border_color: Color, scroll: u16, markdown: &'a str) -> Self {
        Self {
            title,
            border_color,
            scroll,
            markdown,
        }
    }

    pub(super) fn render(self, frame: &mut Frame, area: Rect) -> DetailViewport {
        let text = crate::terminal_md::render_to_tui_text(self.markdown);
        MarkdownPanel::new(self.title, self.border_color, self.scroll, text).render(frame, area)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DetailViewport {
    total_lines: usize,
}

impl DetailViewport {
    pub(super) fn new(total_lines: usize) -> Self {
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

pub(super) struct MetadataLine<'a> {
    label: &'static str,
    value: Vec<Span<'a>>,
}

impl<'a> MetadataLine<'a> {
    pub(super) fn plain(label: &'static str, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label,
            value: vec![Span::raw(value.into())],
        }
    }

    pub(super) fn styled(
        label: &'static str,
        value: impl Into<Cow<'a, str>>,
        style: Style,
    ) -> Self {
        Self {
            label,
            value: vec![Span::styled(value.into(), style)],
        }
    }

    pub(super) fn status(label: &'static str, status: &str) -> Self {
        Self {
            label,
            value: vec![
                Span::styled(format!("{} ", status_icon(status)), status_style(status)),
                Span::styled(status.to_string(), status_style(status)),
            ],
        }
    }

    pub(super) fn phase(label: &'static str, phase: &str) -> Self {
        Self {
            label,
            value: vec![Span::styled(phase.to_string(), phase_style(phase))],
        }
    }

    pub(super) fn joined(label: &'static str, values: &[String], separator: &str) -> Self {
        Self::plain(label, values.join(separator))
    }

    pub(super) fn tags(label: &'static str, tags: &[String]) -> Self {
        Self {
            label,
            value: vec![Span::styled(
                tags.join("  "),
                Style::default().fg(Color::Magenta).bold(),
            )],
        }
    }

    pub(super) fn render(mut self) -> Line<'a> {
        let mut spans = vec![Span::styled(
            self.label,
            Style::default().fg(Color::DarkGray),
        )];
        spans.append(&mut self.value);
        Line::from(spans)
    }
}

pub(super) struct SummaryCard {
    title: &'static str,
    border_color: Color,
    lines: Vec<Line<'static>>,
}

impl SummaryCard {
    pub(super) fn new(
        title: &'static str,
        border_color: Color,
        mut lines: Vec<Line<'static>>,
    ) -> Self {
        lines.insert(0, Line::from(""));
        Self {
            title,
            border_color,
            lines,
        }
    }

    pub(super) fn into_paragraph(self) -> Paragraph<'static> {
        Paragraph::new(self.lines)
            .block(rounded_block(self.title).border_style(Style::default().fg(self.border_color)))
    }
}

#[cfg(test)]
mod tests {
    use super::DetailViewport;

    #[test]
    fn detail_viewport_footer_status_clamps_scroll() {
        let viewport = DetailViewport::new(4);
        let mut scroll = 8;

        assert_eq!(viewport.footer_status(&mut scroll), "Scroll 4/4");
        assert_eq!(scroll, 3);
    }
}
