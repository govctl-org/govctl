use super::super::rounded_block;
use ratatui::{prelude::*, widgets::Paragraph};

pub(in crate::tui::ui) struct SummaryCard {
    title: &'static str,
    border_color: Color,
    metrics: Vec<SummaryMetric>,
    total: usize,
}

impl SummaryCard {
    pub(in crate::tui::ui) fn new(
        title: &'static str,
        border_color: Color,
        metrics: Vec<SummaryMetric>,
        total: usize,
    ) -> Self {
        Self {
            title,
            border_color,
            metrics,
            total,
        }
    }

    pub(in crate::tui::ui) fn into_paragraph(self) -> Paragraph<'static> {
        let title = self.title;
        let border_color = self.border_color;
        Paragraph::new(self.lines())
            .block(rounded_block(title).border_style(Style::default().fg(border_color)))
    }

    fn lines(self) -> Vec<Line<'static>> {
        let mut lines = Vec::with_capacity(self.metrics.len() + 3);
        lines.push(Line::from(""));
        lines.extend(self.metrics.into_iter().map(SummaryMetric::render));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Σ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(format!(" Total: {}", self.total), Style::default().bold()),
        ]));
        lines
    }
}

pub(in crate::tui::ui) struct SummaryMetric {
    icon: &'static str,
    icon_color: Color,
    label: &'static str,
    value: usize,
}

impl SummaryMetric {
    pub(in crate::tui::ui) fn new(
        icon: &'static str,
        icon_color: Color,
        label: &'static str,
        value: usize,
    ) -> Self {
        Self {
            icon,
            icon_color,
            label,
            value,
        }
    }

    fn render(self) -> Line<'static> {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(self.icon, Style::default().fg(self.icon_color)),
            Span::raw(format!(" {}{}", self.label, self.value)),
        ])
    }
}
