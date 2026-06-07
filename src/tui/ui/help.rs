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
            lines.push(Line::from("  r      RFC list"));
            lines.push(Line::from("  c      Clause list"));
            lines.push(Line::from("  a      ADR list"));
            lines.push(Line::from("  w      Work list"));
            lines.push(Line::from("  g      Guard list"));
            lines.push(Line::from("  s      Search"));
            lines.push(Line::from("  l      Loop list"));
            lines.push(Line::from("  d      Diagnostics"));
            lines.push(Line::from("  9      Releases"));
            lines.push(Line::from("  t      Tags"));
        }
        View::RfcList
        | View::ClauseList
        | View::AdrList
        | View::WorkList
        | View::GuardList
        | View::ReleaseList
        | View::TagList
        | View::LoopList
        | View::DiagnosticList => {
            lines.push(Line::from("List"));
            lines.push(Line::from("  j/k    Move selection"));
            lines.push(Line::from("  Enter  View detail"));
            lines.push(Line::from("  g/G    Top/Bottom"));
            lines.push(Line::from("  /      Filter"));
            lines.push(Line::from("  n/p    Next/Prev match (when filtered)"));
            lines.push(Line::from("  Esc    Back (or clear filter in filter mode)"));
        }
        View::Search => {
            lines.push(Line::from("Search"));
            lines.push(Line::from("  e or / Edit query"));
            lines.push(Line::from("  Enter  View selected result"));
            lines.push(Line::from("  j/k    Move selection"));
            lines.push(Line::from("  Esc    Back"));
        }
        View::LoopDetail(_) => {
            lines.push(Line::from("Loop DAG"));
            lines.push(Line::from("  j/k    Select work item"));
            lines.push(Line::from("  Esc    Back"));
        }
        View::RfcDetail(_) => {
            lines.push(Line::from("RFC Detail"));
            lines.push(Line::from("  j/k    Move clause selection"));
            lines.push(Line::from("  Enter  View clause"));
            lines.push(Line::from("  Esc    Back"));
        }
        View::AdrDetail(_)
        | View::WorkDetail(_)
        | View::GuardDetail(_)
        | View::ClauseDetail(_, _) => {
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

#[cfg(test)]
mod tests {
    use super::super::test_support::{project_index, render_app};
    use super::*;

    #[test]
    fn centered_rect_returns_inner_popup_area() {
        let area = Rect::new(0, 0, 100, 40);
        let rect = centered_rect(70, 50, area);

        assert_eq!(rect.width, 70);
        assert_eq!(rect.height, 20);
        assert_eq!(rect.x, 15);
        assert_eq!(rect.y, 10);
    }

    #[test]
    fn help_overlay_renders_view_specific_sections() -> Result<(), Box<dyn std::error::Error>> {
        for (view, expected) in [
            (View::Dashboard, "Dashboard"),
            (View::RfcList, "List"),
            (View::Search, "Search"),
            (View::LoopDetail(0), "Loop DAG"),
            (View::RfcDetail(0), "RFC Detail"),
            (View::WorkDetail(0), "Detail"),
        ] {
            let mut app = App::new(project_index(vec![], vec![], vec![]));
            app.view = view;
            let (_, rendered) = render_app(100, 30, app, |frame, app| {
                draw_overlay(frame, app);
            })?;

            assert!(rendered.iter().any(|line| line.contains("Global")));
            assert!(rendered.iter().any(|line| line.contains(expected)));
        }
        Ok(())
    }
}
