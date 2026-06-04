use super::super::app::{App, View};
use super::rounded_block;
use ratatui::{
    prelude::*,
    widgets::{Clear, Paragraph, Wrap},
};

pub(super) fn draw_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);

    let title = "Help";
    let block = rounded_block(title).border_style(Style::default().fg(Color::Cyan));

    let mut lines = vec![
        Line::from("Global"),
        Line::from("  ?      Toggle help"),
        Line::from("  q      Quit"),
        Line::from(""),
    ];

    match app.view {
        View::Dashboard => {
            lines.push(Line::from("Dashboard"));
            lines.push(Line::from("  1/r    RFC list"));
            lines.push(Line::from("  2/a    ADR list"));
            lines.push(Line::from("  3/w    Work list"));
        }
        View::RfcList | View::AdrList | View::WorkList => {
            lines.push(Line::from("List"));
            lines.push(Line::from("  j/k    Move selection"));
            lines.push(Line::from("  Enter  View detail"));
            lines.push(Line::from("  g/G    Top/Bottom"));
            lines.push(Line::from("  /      Filter"));
            lines.push(Line::from("  n/p    Next/Prev match (when filtered)"));
            lines.push(Line::from("  Esc    Back (or clear filter in filter mode)"));
        }
        View::RfcDetail(_) => {
            lines.push(Line::from("RFC Detail"));
            lines.push(Line::from("  j/k    Move clause selection"));
            lines.push(Line::from("  Enter  View clause"));
            lines.push(Line::from("  Esc    Back"));
        }
        View::AdrDetail(_) | View::WorkDetail(_) | View::ClauseDetail(_, _) => {
            lines.push(Line::from("Detail"));
            lines.push(Line::from("  j/k      Scroll line"));
            lines.push(Line::from("  Ctrl+d/u Half-page"));
            lines.push(Line::from("  PgDn/Up  Full page"));
            lines.push(Line::from("  Esc      Back"));
        }
    }

    let content = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(content, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}
