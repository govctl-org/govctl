use super::super::{phase_style, rounded_block, wrapped_line_count};
use super::resource::StatusText;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use std::borrow::Cow;

pub(in crate::tui::ui) struct MetadataPanel<'a> {
    title: String,
    border_color: Color,
    lines: Vec<Line<'a>>,
}

impl<'a> MetadataPanel<'a> {
    pub(in crate::tui::ui) fn new(
        title: impl Into<String>,
        border_color: Color,
        lines: Vec<Line<'a>>,
    ) -> Self {
        Self {
            title: title.into(),
            border_color,
            lines,
        }
    }

    pub(in crate::tui::ui) fn outer_height(&self) -> u16 {
        (self.lines.len() as u16) + 2
    }

    pub(in crate::tui::ui) fn render(self, frame: &mut Frame, area: Rect) {
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

pub(in crate::tui::ui) struct MarkdownDetailPanel<'a> {
    title: &'a str,
    border_color: Color,
    scroll: u16,
    markdown: &'a str,
}

impl<'a> MarkdownDetailPanel<'a> {
    pub(in crate::tui::ui) fn new(
        title: &'a str,
        border_color: Color,
        scroll: u16,
        markdown: &'a str,
    ) -> Self {
        Self {
            title,
            border_color,
            scroll,
            markdown,
        }
    }

    pub(in crate::tui::ui) fn render(self, frame: &mut Frame, area: Rect) -> DetailViewport {
        let text = crate::terminal_md::render_to_tui_text(self.markdown);
        MarkdownPanel::new(self.title, self.border_color, self.scroll, text).render(frame, area)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui::ui) struct DetailViewport {
    total_lines: usize,
}

impl DetailViewport {
    pub(in crate::tui::ui) fn new(total_lines: usize) -> Self {
        Self { total_lines }
    }

    pub(in crate::tui::ui) fn footer_status(self, scroll: &mut u16) -> String {
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

pub(in crate::tui::ui) struct MetadataLine<'a> {
    label: &'static str,
    value: Vec<Span<'a>>,
}

impl<'a> MetadataLine<'a> {
    pub(in crate::tui::ui) fn plain(label: &'static str, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label,
            value: vec![Span::raw(value.into())],
        }
    }

    pub(in crate::tui::ui) fn styled(
        label: &'static str,
        value: impl Into<Cow<'a, str>>,
        style: Style,
    ) -> Self {
        Self {
            label,
            value: vec![Span::styled(value.into(), style)],
        }
    }

    pub(in crate::tui::ui) fn status(label: &'static str, status: &str) -> Self {
        Self {
            label,
            value: StatusText::new(status).spans(),
        }
    }

    pub(in crate::tui::ui) fn phase(label: &'static str, phase: &str) -> Self {
        Self {
            label,
            value: vec![Span::styled(phase.to_string(), phase_style(phase))],
        }
    }

    pub(in crate::tui::ui) fn joined(
        label: &'static str,
        values: &[String],
        separator: &str,
    ) -> Self {
        Self::plain(label, values.join(separator))
    }

    pub(in crate::tui::ui) fn tags(label: &'static str, tags: &[String]) -> Self {
        Self {
            label,
            value: vec![Span::styled(
                tags.join("  "),
                Style::default().fg(Color::Magenta).bold(),
            )],
        }
    }

    pub(in crate::tui::ui) fn render(mut self) -> Line<'a> {
        let mut spans = vec![Span::styled(
            self.label,
            Style::default().fg(Color::DarkGray),
        )];
        spans.append(&mut self.value);
        Line::from(spans)
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
