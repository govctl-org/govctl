//! UI rendering for TUI.

mod dashboard;
mod lists;
#[cfg(test)]
mod test_support;

use super::app::{App, View};
use crate::theme::{phase_semantic, status_icon, status_semantic};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

fn status_style(status: &str) -> Style {
    Style::default().fg(status_semantic(status).to_ratatui())
}

fn phase_style(phase: &str) -> Style {
    Style::default().fg(phase_semantic(phase).to_ratatui())
}

fn breadcrumb(app: &App) -> String {
    match app.view {
        View::Dashboard => "Dashboard".to_string(),
        View::RfcList => "Dashboard > RFCs".to_string(),
        View::AdrList => "Dashboard > ADRs".to_string(),
        View::WorkList => "Dashboard > Work".to_string(),
        View::RfcDetail(idx) => app
            .index
            .rfcs
            .get(idx)
            .map(|rfc| format!("Dashboard > RFCs > {}", rfc.rfc.rfc_id))
            .unwrap_or_else(|| "Dashboard > RFCs".to_string()),
        View::AdrDetail(idx) => app
            .index
            .adrs
            .get(idx)
            .map(|adr| format!("Dashboard > ADRs > {}", adr.meta().id))
            .unwrap_or_else(|| "Dashboard > ADRs".to_string()),
        View::WorkDetail(idx) => app
            .index
            .work_items
            .get(idx)
            .map(|item| format!("Dashboard > Work > {}", item.meta().id))
            .unwrap_or_else(|| "Dashboard > Work".to_string()),
        View::ClauseDetail(rfc_idx, clause_idx) => app
            .index
            .rfcs
            .get(rfc_idx)
            .and_then(|rfc| rfc.clauses.get(clause_idx).map(|clause| (rfc, clause)))
            .map(|(rfc, clause)| {
                format!(
                    "Dashboard > RFCs > {} > {}",
                    rfc.rfc.rfc_id, clause.spec.clause_id
                )
            })
            .unwrap_or_else(|| "Dashboard > RFCs".to_string()),
    }
}

fn header_status(app: &mut App) -> String {
    match app.view {
        View::Dashboard => format!(
            "RFC {} | ADR {} | Work {}",
            app.index.rfcs.len(),
            app.index.adrs.len(),
            app.index.work_items.len()
        ),
        View::RfcList | View::AdrList | View::WorkList => {
            let total = app.list_total_len();
            let shown = app.list_len();
            let mut parts = vec![format!("Shown {}/{}", shown, total)];
            if shown > 0 {
                parts.push(format!("Sel {}/{}", app.selected + 1, shown));
            }
            if app.filter_mode {
                parts.push(format!("Filter: /{}_", app.filter_query));
            } else if app.filter_active() {
                parts.push(format!("Filter: {}", app.filter_query));
            }
            parts.join(" | ")
        }
        _ => String::new(),
    }
}

// Implements [[RFC-0003:C-NAV]]
fn draw_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(30)])
        .split(inner);

    let left = Paragraph::new(Line::from(vec![
        Span::styled("govctl", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" "),
        Span::raw(breadcrumb(app)),
    ]))
    .alignment(Alignment::Left);

    let right = Paragraph::new(header_status(app)).alignment(Alignment::Right);

    frame.render_widget(left, chunks[0]);
    frame.render_widget(right, chunks[1]);
}

fn keybind_line(bindings: &[&str]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for chunk in bindings.chunks(2) {
        if chunk.len() == 2 {
            spans.push(Span::styled("[", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                chunk[0].to_string(),
                Style::default().fg(Color::Cyan).bold(),
            ));
            spans.push(Span::styled("] ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("{}  ", chunk[1]),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
    Line::from(spans)
}

// Implements [[RFC-0003:C-NAV]]
fn draw_footer(frame: &mut Frame, area: Rect, bindings: &[&str], status: Option<&str>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(30)])
        .split(inner);

    let left = Paragraph::new(keybind_line(bindings)).alignment(Alignment::Center);
    let right = Paragraph::new(status.unwrap_or("")).alignment(Alignment::Right);

    frame.render_widget(left, chunks[0]);
    frame.render_widget(right, chunks[1]);
}

/// Main draw function
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(area);

    draw_header(frame, app, chunks[0]);
    app.content_height = chunks[1].height;

    let mut footer_status = None;
    let bindings: &[&str] = match app.view {
        View::Dashboard => &[
            "1/r", "RFCs", "2/a", "ADRs", "3/w", "Work", "?", "Help", "q", "Quit",
        ],
        View::RfcList | View::AdrList | View::WorkList => &[
            "j/k", "Navigate", "Enter", "View", "Esc", "Back", "/", "Filter", "g/G", "Jump", "?",
            "Help", "q", "Quit",
        ],
        View::RfcDetail(_) => &[
            "j/k",
            "Navigate",
            "Enter",
            "View Clause",
            "Esc",
            "Back",
            "?",
            "Help",
            "q",
            "Quit",
        ],
        View::AdrDetail(_) | View::WorkDetail(_) | View::ClauseDetail(_, _) => &[
            "j/k", "Scroll", "^d/^u", "Page", "Esc", "Back", "?", "Help", "q", "Quit",
        ],
    };

    match app.view {
        View::Dashboard => dashboard::draw(frame, app, chunks[1]),
        View::RfcList => lists::draw_rfc(frame, app, chunks[1]),
        View::AdrList => lists::draw_adr(frame, app, chunks[1]),
        View::WorkList => lists::draw_work(frame, app, chunks[1]),
        View::RfcDetail(idx) => draw_rfc_detail(frame, app, chunks[1], idx),
        View::AdrDetail(idx) => {
            // Implements [[RFC-0003:C-DETAIL]]
            let total = draw_adr_detail(frame, app, chunks[1], idx);
            let max_scroll = total.saturating_sub(1) as u16;
            if app.scroll > max_scroll {
                app.scroll = max_scroll;
            }
            footer_status = Some(format!("Scroll {}/{}", app.scroll + 1, total));
        }
        View::WorkDetail(idx) => {
            // Implements [[RFC-0003:C-DETAIL]]
            let total = draw_work_detail(frame, app, chunks[1], idx);
            let max_scroll = total.saturating_sub(1) as u16;
            if app.scroll > max_scroll {
                app.scroll = max_scroll;
            }
            footer_status = Some(format!("Scroll {}/{}", app.scroll + 1, total));
        }
        View::ClauseDetail(rfc_idx, clause_idx) => {
            // Implements [[RFC-0003:C-DETAIL]]
            let total = draw_clause_detail(frame, app, chunks[1], rfc_idx, clause_idx);
            let max_scroll = total.saturating_sub(1) as u16;
            if app.scroll > max_scroll {
                app.scroll = max_scroll;
            }
            footer_status = Some(format!("Scroll {}/{}", app.scroll + 1, total));
        }
    }

    draw_footer(frame, chunks[2], bindings, footer_status.as_deref());

    if app.show_help {
        draw_help_overlay(frame, app);
    }
}

/// Estimate the number of rendered lines after word-wrap.
///
/// Implements [[RFC-0003:C-DETAIL]] scroll position accuracy.
fn wrapped_line_count(lines: &[Line], render_width: u16) -> usize {
    if render_width == 0 {
        return lines.len();
    }
    let w = render_width as usize;
    lines
        .iter()
        .map(|line| {
            let display_width = line.width();
            if display_width == 0 {
                1
            } else {
                display_width.div_ceil(w)
            }
        })
        .sum()
}

fn rounded_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
}

fn draw_rfc_detail(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) {
    let Some(rfc) = app.index.rfcs.get(idx) else {
        return;
    };

    let status = rfc.rfc.status.as_ref();
    let phase = rfc.rfc.phase.as_ref();

    let mut header_lines = vec![
        Line::from(vec![
            Span::styled("ID:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(rfc.rfc.rfc_id.clone(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Title:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(rfc.rfc.title.clone()),
        ]),
        Line::from(vec![
            Span::styled("Version: ", Style::default().fg(Color::DarkGray)),
            Span::styled(rfc.rfc.version.clone(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Status:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", status_icon(status)), status_style(status)),
            Span::styled(status.to_string(), status_style(status)),
        ]),
        Line::from(vec![
            Span::styled("Phase:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(phase.to_string(), phase_style(phase)),
        ]),
        Line::from(vec![
            Span::styled("Owners:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(rfc.rfc.owners.join(", ")),
        ]),
    ];

    if !rfc.rfc.refs.is_empty() {
        header_lines.push(Line::from(vec![
            Span::styled("Refs:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(rfc.rfc.refs.join(", ")),
        ]));
    }

    if !rfc.rfc.tags.is_empty() {
        header_lines.push(Line::from(vec![
            Span::styled("Tags:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                rfc.rfc.tags.join("  "),
                Style::default().fg(Color::Magenta).bold(),
            ),
        ]));
    }

    // +2 for block borders
    let header_height = (header_lines.len() as u16) + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(5)])
        .split(area);

    let title = format!("📋 {}", rfc.rfc.rfc_id);
    let header = Paragraph::new(header_lines)
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Blue)));
    frame.render_widget(header, chunks[0]);

    // Clause list using List widget
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

fn draw_adr_detail(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> usize {
    let Some(adr) = app.index.adrs.get(idx) else {
        return 0;
    };

    let text = crate::render::render_adr(adr)
        .map(|md| crate::terminal_md::render_to_tui_text(&md))
        .unwrap_or_default();

    let title = format!("📝 {}", adr.meta().id);
    let block = rounded_block(&title).border_style(Style::default().fg(Color::Green));
    let inner_width = block.inner(area).width;
    let total_lines = wrapped_line_count(&text.lines, inner_width);
    let content = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(block);

    frame.render_widget(content, area);
    total_lines
}

fn draw_work_detail(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> usize {
    let Some(item) = app.index.work_items.get(idx) else {
        return 0;
    };

    let text = crate::render::render_work_item(item)
        .map(|md| crate::terminal_md::render_to_tui_text(&md))
        .unwrap_or_default();

    let title = format!("📌 {}", item.meta().id);
    let block = rounded_block(&title).border_style(Style::default().fg(Color::Yellow));
    let inner_width = block.inner(area).width;
    let total_lines = wrapped_line_count(&text.lines, inner_width);
    let content = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(block);

    frame.render_widget(content, area);
    total_lines
}

fn draw_clause_detail(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    rfc_idx: usize,
    clause_idx: usize,
) -> usize {
    let Some(rfc) = app.index.rfcs.get(rfc_idx) else {
        return 0;
    };

    let Some(clause) = rfc.clauses.get(clause_idx) else {
        return 0;
    };

    let mut raw = String::new();
    crate::render::render_clause(&mut raw, &rfc.rfc.rfc_id, clause);
    let text = crate::terminal_md::render_to_tui_text(&raw);

    let title = format!("📜 {}", clause.spec.clause_id);
    let block = rounded_block(&title).border_style(Style::default().fg(Color::Magenta));
    let inner_width = block.inner(area).width;
    let total_lines = wrapped_line_count(&text.lines, inner_width);
    let content = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(block);

    frame.render_widget(content, area);
    total_lines
}

fn draw_help_overlay(frame: &mut Frame, app: &App) {
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
