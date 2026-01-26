//! UI rendering for TUI.

use super::app::{App, View};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
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

/// Main draw function
pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Dashboard => draw_dashboard(frame, app),
        View::RfcList => draw_rfc_list(frame, app),
        View::AdrList => draw_adr_list(frame, app),
        View::WorkList => draw_work_list(frame, app),
        View::RfcDetail(idx) => draw_rfc_detail(frame, app, idx),
        View::AdrDetail(idx) => draw_adr_detail(frame, app, idx),
        View::WorkDetail(idx) => draw_work_detail(frame, app, idx),
    }
}

fn rounded_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
}

fn draw_dashboard(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Split into header, content, and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Header with fancy title
    let header = Paragraph::new(Line::from(vec![
        Span::styled("╔══ ", Style::default().fg(Color::Cyan)),
        Span::styled("govctl", Style::default().fg(Color::Cyan).bold()),
        Span::styled(" Dashboard ══╗", Style::default().fg(Color::Cyan)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, chunks[0]);

    // Content: 3 columns for RFC, ADR, Work
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    // RFC stats
    let rfc_stats = build_rfc_stats(app);
    frame.render_widget(rfc_stats, content_chunks[0]);

    // ADR stats
    let adr_stats = build_adr_stats(app);
    frame.render_widget(adr_stats, content_chunks[1]);

    // Work stats
    let work_stats = build_work_stats(app);
    frame.render_widget(work_stats, content_chunks[2]);

    // Footer with keybindings
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" [", Style::default().fg(Color::DarkGray)),
        Span::styled("1", Style::default().fg(Color::Blue).bold()),
        Span::styled("/r] RFCs  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled("2", Style::default().fg(Color::Green).bold()),
        Span::styled("/a] ADRs  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled("3", Style::default().fg(Color::Yellow).bold()),
        Span::styled("/w] Work  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::Red).bold()),
        Span::styled("] Quit ", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(footer, chunks[2]);
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

fn draw_rfc_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    // Build table rows with colored status
    let rows: Vec<Row> = app
        .index
        .rfcs
        .iter()
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

    frame.render_stateful_widget(table, chunks[0], &mut app.table_state);

    let footer = keybind_footer(&[
        "j/k", "Navigate", "Enter", "View", "Esc", "Back", "q", "Quit",
    ]);
    frame.render_widget(footer, chunks[1]);
}

fn draw_adr_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let rows: Vec<Row> = app
        .index
        .adrs
        .iter()
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

    frame.render_stateful_widget(table, chunks[0], &mut app.table_state);

    let footer = keybind_footer(&[
        "j/k", "Navigate", "Enter", "View", "Esc", "Back", "q", "Quit",
    ]);
    frame.render_widget(footer, chunks[1]);
}

fn draw_work_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let rows: Vec<Row> = app
        .index
        .work_items
        .iter()
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

    frame.render_stateful_widget(table, chunks[0], &mut app.table_state);

    let footer = keybind_footer(&[
        "j/k", "Navigate", "Enter", "View", "Esc", "Back", "q", "Quit",
    ]);
    frame.render_widget(footer, chunks[1]);
}

fn keybind_footer(bindings: &[&str]) -> Paragraph<'static> {
    let mut spans = vec![Span::raw(" ")];
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

    Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
}

fn draw_rfc_detail(frame: &mut Frame, app: &mut App, idx: usize) {
    let area = frame.area();

    let Some(rfc) = app.index.rfcs.get(idx) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let status = rfc.rfc.status.as_ref();
    let phase = rfc.rfc.phase.as_ref();

    let mut lines = vec![
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
        Line::from(""),
        Line::from(Span::styled(
            "━━━ Clauses ━━━",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
    ];

    for clause in &rfc.clauses {
        let clause_status = clause.spec.status.as_ref();
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
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
        ]));
    }

    let title = format!("📋 {}", rfc.rfc.rfc_id);
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Blue)));

    frame.render_widget(content, chunks[0]);

    let footer = keybind_footer(&["j/k", "Scroll", "Esc", "Back", "q", "Quit"]);
    frame.render_widget(footer, chunks[1]);
}

fn draw_adr_detail(frame: &mut Frame, app: &mut App, idx: usize) {
    let area = frame.area();

    let Some(adr) = app.index.adrs.get(idx) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
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
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Green)));

    frame.render_widget(content, chunks[0]);

    let footer = keybind_footer(&["j/k", "Scroll", "Esc", "Back", "q", "Quit"]);
    frame.render_widget(footer, chunks[1]);
}

fn draw_work_detail(frame: &mut Frame, app: &mut App, idx: usize) {
    let area = frame.area();

    let Some(item) = app.index.work_items.get(idx) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
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
    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(rounded_block(&title).border_style(Style::default().fg(Color::Yellow)));

    frame.render_widget(content, chunks[0]);

    let footer = keybind_footer(&["j/k", "Scroll", "Esc", "Back", "q", "Quit"]);
    frame.render_widget(footer, chunks[1]);
}
