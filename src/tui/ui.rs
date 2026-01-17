//! UI rendering for TUI.

use super::app::{App, View};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Row, Table},
};

/// Main draw function
pub fn draw(frame: &mut Frame, app: &App) {
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

fn draw_dashboard(frame: &mut Frame, app: &App) {
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

    // Header
    let header = Paragraph::new("govctl Dashboard")
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
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

    // Footer
    let footer = Paragraph::new("[1/r] RFCs  [2/a] ADRs  [3/w] Work  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

fn build_rfc_stats(app: &App) -> Paragraph<'static> {
    let mut lines = vec![Line::from("").style(Style::default())];

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

    lines.push(Line::from(format!("  Draft:      {}", draft)));
    lines.push(Line::from(format!("  Normative:  {}", normative)));
    lines.push(Line::from(format!("  Deprecated: {}", deprecated)));
    lines.push(Line::from(""));
    lines.push(Line::from(format!("  Total: {}", app.index.rfcs.len())));

    Paragraph::new(lines).block(
        Block::default()
            .title(" RFCs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    )
}

fn build_adr_stats(app: &App) -> Paragraph<'static> {
    let mut lines = vec![Line::from("").style(Style::default())];

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

    lines.push(Line::from(format!("  Proposed:   {}", proposed)));
    lines.push(Line::from(format!("  Accepted:   {}", accepted)));
    lines.push(Line::from(format!("  Superseded: {}", superseded)));
    lines.push(Line::from(""));
    lines.push(Line::from(format!("  Total: {}", app.index.adrs.len())));

    Paragraph::new(lines).block(
        Block::default()
            .title(" ADRs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    )
}

fn build_work_stats(app: &App) -> Paragraph<'static> {
    let mut lines = vec![Line::from("").style(Style::default())];

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

    lines.push(Line::from(format!("  Queue:  {}", queue)));
    lines.push(Line::from(format!("  Active: {}", active)));
    lines.push(Line::from(format!("  Done:   {}", done)));
    lines.push(Line::from(""));
    lines.push(Line::from(format!(
        "  Total: {}",
        app.index.work_items.len()
    )));

    Paragraph::new(lines).block(
        Block::default()
            .title(" Work Items ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    )
}

fn draw_rfc_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    // Build table rows
    let rows: Vec<Row> = app
        .index
        .rfcs
        .iter()
        .enumerate()
        .map(|(i, rfc)| {
            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                rfc.rfc.rfc_id.clone(),
                rfc.rfc.title.clone(),
                rfc.rfc.status.as_ref().to_string(),
                rfc.rfc.phase.as_ref().to_string(),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Min(30),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["ID", "Title", "Status", "Phase"])
            .style(Style::default().bold().fg(Color::Cyan)),
    )
    .block(Block::default().title(" RFCs ").borders(Borders::ALL));

    frame.render_widget(table, chunks[0]);

    let footer = Paragraph::new("[j/k] Navigate  [Enter] View  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[1]);
}

fn draw_adr_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let rows: Vec<Row> = app
        .index
        .adrs
        .iter()
        .enumerate()
        .map(|(i, adr)| {
            let meta = adr.meta();
            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                meta.id.clone(),
                meta.title.clone(),
                meta.status.as_ref().to_string(),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(40),
            Constraint::Length(12),
        ],
    )
    .header(Row::new(vec!["ID", "Title", "Status"]).style(Style::default().bold().fg(Color::Cyan)))
    .block(Block::default().title(" ADRs ").borders(Borders::ALL));

    frame.render_widget(table, chunks[0]);

    let footer = Paragraph::new("[j/k] Navigate  [Enter] View  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[1]);
}

fn draw_work_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let rows: Vec<Row> = app
        .index
        .work_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let meta = item.meta();
            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                meta.id.clone(),
                meta.title.clone(),
                meta.status.as_ref().to_string(),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Min(40),
            Constraint::Length(10),
        ],
    )
    .header(Row::new(vec!["ID", "Title", "Status"]).style(Style::default().bold().fg(Color::Cyan)))
    .block(Block::default().title(" Work Items ").borders(Borders::ALL));

    frame.render_widget(table, chunks[0]);

    let footer = Paragraph::new("[j/k] Navigate  [Enter] View  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[1]);
}

fn draw_rfc_detail(frame: &mut Frame, app: &App, idx: usize) {
    let area = frame.area();

    let Some(rfc) = app.index.rfcs.get(idx) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let mut lines = vec![
        Line::from(format!("ID:      {}", rfc.rfc.rfc_id)),
        Line::from(format!("Title:   {}", rfc.rfc.title)),
        Line::from(format!("Version: {}", rfc.rfc.version)),
        Line::from(format!("Status:  {}", rfc.rfc.status.as_ref())),
        Line::from(format!("Phase:   {}", rfc.rfc.phase.as_ref())),
        Line::from(format!("Owners:  {}", rfc.rfc.owners.join(", "))),
        Line::from(""),
        Line::from("Clauses:").style(Style::default().bold()),
    ];

    for clause in &rfc.clauses {
        lines.push(Line::from(format!(
            "  {} - {} [{}]",
            clause.spec.clause_id,
            clause.spec.title,
            clause.spec.status.as_ref()
        )));
    }

    let content = Paragraph::new(lines).scroll((app.scroll, 0)).block(
        Block::default()
            .title(format!(" {} ", rfc.rfc.rfc_id))
            .borders(Borders::ALL),
    );

    frame.render_widget(content, chunks[0]);

    let footer = Paragraph::new("[j/k] Scroll  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[1]);
}

fn draw_adr_detail(frame: &mut Frame, app: &App, idx: usize) {
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

    let lines = vec![
        Line::from(format!("ID:     {}", meta.id)),
        Line::from(format!("Title:  {}", meta.title)),
        Line::from(format!("Status: {}", meta.status.as_ref())),
        Line::from(format!("Date:   {}", meta.date)),
        Line::from(""),
        Line::from("Context:").style(Style::default().bold()),
        Line::from(format!("  {}", content_data.context)),
        Line::from(""),
        Line::from("Decision:").style(Style::default().bold()),
        Line::from(format!("  {}", content_data.decision)),
        Line::from(""),
        Line::from("Consequences:").style(Style::default().bold()),
        Line::from(format!("  {}", content_data.consequences)),
    ];

    let content = Paragraph::new(lines).scroll((app.scroll, 0)).block(
        Block::default()
            .title(format!(" {} ", meta.id))
            .borders(Borders::ALL),
    );

    frame.render_widget(content, chunks[0]);

    let footer = Paragraph::new("[j/k] Scroll  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[1]);
}

fn draw_work_detail(frame: &mut Frame, app: &App, idx: usize) {
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

    let mut lines = vec![
        Line::from(format!("ID:     {}", meta.id)),
        Line::from(format!("Title:  {}", meta.title)),
        Line::from(format!("Status: {}", meta.status.as_ref())),
        Line::from(""),
        Line::from("Description:").style(Style::default().bold()),
        Line::from(format!("  {}", content_data.description)),
    ];

    if !content_data.acceptance_criteria.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Acceptance Criteria:").style(Style::default().bold()));
        for ac in &content_data.acceptance_criteria {
            let checkbox = match ac.status {
                crate::model::ChecklistStatus::Done => "[x]",
                crate::model::ChecklistStatus::Cancelled => "[-]",
                crate::model::ChecklistStatus::Pending => "[ ]",
            };
            lines.push(Line::from(format!("  {} {}", checkbox, ac.text)));
        }
    }

    let content = Paragraph::new(lines).scroll((app.scroll, 0)).block(
        Block::default()
            .title(format!(" {} ", meta.id))
            .borders(Borders::ALL),
    );

    frame.render_widget(content, chunks[0]);

    let footer = Paragraph::new("[j/k] Scroll  [Esc] Back  [q] Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[1]);
}
