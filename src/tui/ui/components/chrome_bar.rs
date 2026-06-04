use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Borders, Paragraph},
};

pub(in crate::tui::ui) struct ChromeBar {
    border_color: Color,
    left: Line<'static>,
    left_alignment: Alignment,
    right: String,
}

impl ChromeBar {
    pub(in crate::tui::ui) fn new(
        border_color: Color,
        left: Line<'static>,
        right: impl Into<String>,
    ) -> Self {
        Self {
            border_color,
            left,
            left_alignment: Alignment::Left,
            right: right.into(),
        }
    }

    pub(in crate::tui::ui) fn left_alignment(mut self, alignment: Alignment) -> Self {
        self.left_alignment = alignment;
        self
    }

    pub(in crate::tui::ui) fn render(self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(self.border_color));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(30)])
            .split(inner);

        let left = Paragraph::new(self.left).alignment(self.left_alignment);
        let right = Paragraph::new(self.right).alignment(Alignment::Right);

        frame.render_widget(left, chunks[0]);
        frame.render_widget(right, chunks[1]);
    }
}
