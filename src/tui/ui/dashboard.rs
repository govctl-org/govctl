use super::super::app::App;
use super::components::{SummaryCard, SummaryMetric};
use ratatui::{prelude::*, widgets::Paragraph};

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
    title: &'static str,
    border_color: Color,
    metrics: Vec<SummaryMetric>,
    total: usize,
) -> Paragraph<'static> {
    SummaryCard::new(title, border_color, metrics, total).into_paragraph()
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
            SummaryMetric::new("○", Color::Yellow, "Draft:      ", counts[0]),
            SummaryMetric::new("●", Color::Green, "Normative:  ", counts[1]),
            SummaryMetric::new("✗", Color::Red, "Deprecated: ", counts[2]),
        ],
        app.index.rfcs.len(),
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
            SummaryMetric::new("○", Color::Yellow, "Proposed:   ", counts[0]),
            SummaryMetric::new("●", Color::Green, "Accepted:   ", counts[1]),
            SummaryMetric::new("✗", Color::Red, "Superseded: ", counts[2]),
        ],
        app.index.adrs.len(),
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
            SummaryMetric::new("○", Color::Yellow, "Queue:  ", counts[0]),
            SummaryMetric::new("◉", Color::Green, "Active: ", counts[1]),
            SummaryMetric::new("●", Color::Green, "Done:   ", counts[2]),
        ],
        app.index.work_items.len(),
    )
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
