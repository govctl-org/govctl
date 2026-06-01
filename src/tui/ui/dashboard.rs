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

fn count_statuses<T, F, const N: usize>(
    items: &[T],
    statuses: [&str; N],
    mut status_for: F,
) -> [usize; N]
where
    F: for<'a> FnMut(&'a T) -> &'a str,
{
    let mut counts = [0; N];
    for item in items {
        let status = status_for(item);
        if let Some(idx) = statuses.iter().position(|expected| *expected == status) {
            counts[idx] += 1;
        }
    }
    counts
}

fn rfc_stats(app: &App) -> Paragraph<'static> {
    let counts = count_statuses(
        &app.index.rfcs,
        ["draft", "normative", "deprecated"],
        |rfc| rfc.rfc.status.as_ref(),
    );

    summary_block(
        "📋 RFCs",
        Color::Blue,
        vec![
            summary_line("○", Color::Yellow, "Draft:      ", counts[0]),
            summary_line("●", Color::Green, "Normative:  ", counts[1]),
            summary_line("✗", Color::Red, "Deprecated: ", counts[2]),
            Line::from(""),
            total_line(app.index.rfcs.len()),
        ],
    )
}

fn adr_stats(app: &App) -> Paragraph<'static> {
    let counts = count_statuses(
        &app.index.adrs,
        ["proposed", "accepted", "superseded"],
        |adr| adr.meta().status.as_ref(),
    );

    summary_block(
        "📝 ADRs",
        Color::Green,
        vec![
            summary_line("○", Color::Yellow, "Proposed:   ", counts[0]),
            summary_line("●", Color::Green, "Accepted:   ", counts[1]),
            summary_line("✗", Color::Red, "Superseded: ", counts[2]),
            Line::from(""),
            total_line(app.index.adrs.len()),
        ],
    )
}

fn work_stats(app: &App) -> Paragraph<'static> {
    let counts = count_statuses(&app.index.work_items, ["queue", "active", "done"], |item| {
        item.meta().status.as_ref()
    });

    summary_block(
        "📌 Work Items",
        Color::Yellow,
        vec![
            summary_line("○", Color::Yellow, "Queue:  ", counts[0]),
            summary_line("◉", Color::Green, "Active: ", counts[1]),
            summary_line("●", Color::Green, "Done:   ", counts[2]),
            Line::from(""),
            total_line(app.index.work_items.len()),
        ],
    )
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
