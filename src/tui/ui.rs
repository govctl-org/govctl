//! UI rendering for TUI.

use super::app::{App, View};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
};

// Status color helpers
fn status_style(status: &str) -> Style {
    match status {
        "normative" | "accepted" | "done" | "active" => Style::default().fg(Color::Green),
        "draft" | "proposed" | "queue" => Style::default().fg(Color::Yellow),
        "deprecated" | "superseded" | "cancelled" => Style::default().fg(Color::Red),
        _ => Style::default(),
    }
}

fn status_icon(status: &str) -> &'static str {
    match status {
        "normative" | "accepted" | "done" => "●",
        "active" => "◉",
        "draft" | "proposed" | "queue" => "○",
        "deprecated" | "superseded" | "cancelled" => "✗",
        _ => "•",
    }
}

fn phase_style(phase: &str) -> Style {
    match phase {
        "stable" => Style::default().fg(Color::Green),
        "test" => Style::default().fg(Color::Cyan),
        "impl" => Style::default().fg(Color::Blue),
        "spec" => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    }
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

fn header_status(app: &App) -> String {
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
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
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
        View::AdrDetail(_) | View::WorkDetail(_) | View::ClauseDetail(_, _) => {
            &["j/k", "Scroll", "Esc", "Back", "?", "Help", "q", "Quit"]
        }
    };

    match app.view {
        View::Dashboard => draw_dashboard(frame, app, chunks[1]),
        View::RfcList => draw_rfc_list(frame, app, chunks[1]),
        View::AdrList => draw_adr_list(frame, app, chunks[1]),
        View::WorkList => draw_work_list(frame, app, chunks[1]),
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

fn rounded_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
}

fn draw_dashboard(frame: &mut Frame, app: &mut App, area: Rect) {
    // Content: 3 columns for RFC, ADR, Work
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    // RFC stats
    let rfc_stats = build_rfc_stats(app);
    frame.render_widget(rfc_stats, content_chunks[0]);

    // ADR stats
    let adr_stats = build_adr_stats(app);
    frame.render_widget(adr_stats, content_chunks[1]);

    // Work stats
    let work_stats = build_work_stats(app);
    frame.render_widget(work_stats, content_chunks[2]);
}

fn build_rfc_stats(app: &App) -> Paragraph<'static> {
    let mut lines = vec![Line::from("")];

    // Count by status
    let mut draft = 0;
    let mut normative = 0;
    let mut deprecated = 0;

    for rfc in &app.index.rfcs {
        match rfc.rfc.status.as_ref() {
            "draft" => draft += 1,
            "normative" => normative += 1,
            "deprecated" => deprecated += 1,
            _ => {}
        }
    }

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("○", Style::default().fg(Color::Yellow)),
        Span::raw(format!(" Draft:      {}", draft)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("●", Style::default().fg(Color::Green)),
        Span::raw(format!(" Normative:  {}", normative)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("✗", Style::default().fg(Color::Red)),
        Span::raw(format!(" Deprecated: {}", deprecated)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Σ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(
            format!(" Total: {}", app.index.rfcs.len()),
            Style::default().bold(),
        ),
    ]));

    Paragraph::new(lines).block(
        Block::default()
            .title(" 📋 RFCs ")
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Blue)),
    )
}

fn build_adr_stats(app: &App) -> Paragraph<'static> {
    let mut lines = vec![Line::from("")];

    let mut proposed = 0;
    let mut accepted = 0;
    let mut superseded = 0;

    for adr in &app.index.adrs {
        match adr.meta().status.as_ref() {
            "proposed" => proposed += 1,
            "accepted" => accepted += 1,
            "superseded" => superseded += 1,
            _ => {}
        }
    }

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("○", Style::default().fg(Color::Yellow)),
        Span::raw(format!(" Proposed:   {}", proposed)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("●", Style::default().fg(Color::Green)),
        Span::raw(format!(" Accepted:   {}", accepted)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("✗", Style::default().fg(Color::Red)),
        Span::raw(format!(" Superseded: {}", superseded)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Σ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(
            format!(" Total: {}", app.index.adrs.len()),
            Style::default().bold(),
        ),
    ]));

    Paragraph::new(lines).block(
        Block::default()
            .title(" 📝 ADRs ")
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Green)),
    )
}

fn build_work_stats(app: &App) -> Paragraph<'static> {
    let mut lines = vec![Line::from("")];

    let mut queue = 0;
    let mut active = 0;
    let mut done = 0;

    for item in &app.index.work_items {
        match item.meta().status.as_ref() {
            "queue" => queue += 1,
            "active" => active += 1,
            "done" => done += 1,
            _ => {}
        }
    }

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("○", Style::default().fg(Color::Yellow)),
        Span::raw(format!(" Queue:  {}", queue)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("◉", Style::default().fg(Color::Green)),
        Span::raw(format!(" Active: {}", active)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("●", Style::default().fg(Color::Green)),
        Span::raw(format!(" Done:   {}", done)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Σ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(
            format!(" Total: {}", app.index.work_items.len()),
            Style::default().bold(),
        ),
    ]));

    Paragraph::new(lines).block(
        Block::default()
            .title(" 📌 Work Items ")
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Yellow)),
    )
}

fn draw_rfc_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows: Vec<Row> = indices
        .iter()
        .filter_map(|&idx| app.index.rfcs.get(idx))
        .map(|rfc| {
            let status = rfc.rfc.status.as_ref();
            let phase = rfc.rfc.phase.as_ref();

            Row::new(vec![
                Line::from(rfc.rfc.rfc_id.clone()),
                Line::from(rfc.rfc.title.clone()),
                Line::from(vec![
                    Span::styled(format!("{} ", status_icon(status)), status_style(status)),
                    Span::styled(status.to_string(), status_style(status)),
                ]),
                Line::from(Span::styled(phase.to_string(), phase_style(phase))),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Min(30),
            Constraint::Length(14),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["ID", "Title", "Status", "Phase"])
            .style(Style::default().bold().fg(Color::Cyan))
            .bottom_margin(1),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .block(rounded_block("📋 RFCs").border_style(Style::default().fg(Color::Blue)));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_adr_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows: Vec<Row> = indices
        .iter()
        .filter_map(|&idx| app.index.adrs.get(idx))
        .map(|adr| {
            let meta = adr.meta();
            let status = meta.status.as_ref();

            Row::new(vec![
                Line::from(meta.id.clone()),
                Line::from(meta.title.clone()),
                Line::from(vec![
                    Span::styled(format!("{} ", status_icon(status)), status_style(status)),
                    Span::styled(status.to_string(), status_style(status)),
                ]),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(40),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(vec!["ID", "Title", "Status"])
            .style(Style::default().bold().fg(Color::Cyan))
            .bottom_margin(1),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .block(rounded_block("📝 ADRs").border_style(Style::default().fg(Color::Green)));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_work_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.list_indices();
    let rows: Vec<Row> = indices
        .iter()
        .filter_map(|&idx| app.index.work_items.get(idx))
        .map(|item| {
            let meta = item.meta();
            let status = meta.status.as_ref();

            Row::new(vec![
                Line::from(meta.id.clone()),
                Line::from(meta.title.clone()),
                Line::from(vec![
                    Span::styled(format!("{} ", status_icon(status)), status_style(status)),
                    Span::styled(status.to_string(), status_style(status)),
                ]),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Min(40),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["ID", "Title", "Status"])
            .style(Style::default().bold().fg(Color::Cyan))
            .bottom_margin(1),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .block(rounded_block("📌 Work Items").border_style(Style::default().fg(Color::Yellow)));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_rfc_detail(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) {
    let Some(rfc) = app.index.rfcs.get(idx) else {
        return;
    };

    // Split into: header, clause list, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Header (metadata)
            Constraint::Min(5),    // Clause list
        ])
        .split(area);

    let status = rfc.rfc.status.as_ref();
    let phase = rfc.rfc.phase.as_ref();

    // Header with RFC metadata
    let header_lines = vec![
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5)])
        .split(area);

    let meta = adr.meta();
    let content_data = &adr.spec.content;
    let status = meta.status.as_ref();

    let lines = vec![
        Line::from(vec![
            Span::styled("ID:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(meta.id.clone(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Title:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(meta.title.clone()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", status_icon(status)), status_style(status)),
            Span::styled(status.to_string(), status_style(status)),
        ]),
        Line::from(vec![
            Span::styled("Date:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(meta.date.clone()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "━━━ Context ━━━",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(format!("  {}", content_data.context)),
        Line::from(""),
        Line::from(Span::styled(
            "━━━ Decision ━━━",
            Style::default().fg(Color::Green).bold(),
        )),
        Line::from(format!("  {}", content_data.decision)),
        Line::from(""),
        Line::from(Span::styled(
            "━━━ Consequences ━━━",
            Style::default().fg(Color::Yellow).bold(),
        )),
        Line::from(format!("  {}", content_data.consequences)),
    ];

    let title = format!("📝 {}", meta.id);
    let total_lines = lines.len();
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Green)));

    frame.render_widget(content, chunks[0]);
    total_lines
}

fn draw_work_detail(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> usize {
    let Some(item) = app.index.work_items.get(idx) else {
        return 0;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5)])
        .split(area);

    let meta = item.meta();
    let content_data = &item.spec.content;
    let status = meta.status.as_ref();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("ID:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(meta.id.clone(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Title:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(meta.title.clone()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", status_icon(status)), status_style(status)),
            Span::styled(status.to_string(), status_style(status)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "━━━ Description ━━━",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(format!("  {}", content_data.description)),
    ];

    if !content_data.acceptance_criteria.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "━━━ Acceptance Criteria ━━━",
            Style::default().fg(Color::Magenta).bold(),
        )));
        for ac in &content_data.acceptance_criteria {
            let (icon, style) = match ac.status {
                crate::model::ChecklistStatus::Done => ("✓", Style::default().fg(Color::Green)),
                crate::model::ChecklistStatus::Cancelled => ("✗", Style::default().fg(Color::Red)),
                crate::model::ChecklistStatus::Pending => ("○", Style::default().fg(Color::Yellow)),
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{} ", icon), style),
                Span::raw(ac.text.clone()),
            ]));
        }
    }

    let title = format!("📌 {}", meta.id);
    let total_lines = lines.len();
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Yellow)));

    frame.render_widget(content, chunks[0]);
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5)])
        .split(area);

    let status = clause.spec.status.as_ref();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Clause:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(clause.spec.clause_id.clone(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::styled("Title:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(clause.spec.title.clone()),
        ]),
        Line::from(vec![
            Span::styled("Status:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", status_icon(status)), status_style(status)),
            Span::styled(status.to_string(), status_style(status)),
        ]),
        Line::from(vec![
            Span::styled("Kind:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(clause.spec.kind.as_ref().to_string()),
        ]),
        Line::from(vec![
            Span::styled("RFC:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(rfc.rfc.rfc_id.clone(), Style::default().fg(Color::Blue)),
        ]),
    ];

    if let Some(since) = &clause.spec.since {
        lines.push(Line::from(vec![
            Span::styled("Since:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(since.clone()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "━━━ Specification ━━━",
        Style::default().fg(Color::Cyan).bold(),
    )));
    lines.push(Line::from(""));

    // Wrap the clause text
    for line in clause.spec.text.lines() {
        lines.push(Line::from(format!("  {}", line)));
    }

    let title = format!("📜 {}", clause.spec.clause_id);
    let total_lines = lines.len();
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Magenta)));

    frame.render_widget(content, chunks[0]);
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
            lines.push(Line::from("  j/k    Scroll"));
            lines.push(Line::from("  Esc    Back"));
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
