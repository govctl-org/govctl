use super::super::app::App;
use super::{
    components::{ClauseListRow, SelectableList, StatusText},
    panel_block, phase_style, wrapped_line_count,
};
use crate::model::RfcStatus;
use crate::render::RenderProjection;
use crate::tui::dag::dag_lines;
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};
use std::borrow::Cow;

struct MetadataPanel<'a> {
    title: String,
    border_color: Color,
    lines: Vec<Line<'a>>,
}

impl<'a> MetadataPanel<'a> {
    fn new(title: impl Into<String>, border_color: Color, lines: Vec<Line<'a>>) -> Self {
        Self {
            title: title.into(),
            border_color,
            lines,
        }
    }

    fn outer_height(&self) -> u16 {
        (self.lines.len() as u16) + 2
    }

    fn render(self, frame: &mut Frame, area: Rect) {
        let block = panel_block(&self.title).border_style(Style::default().fg(self.border_color));
        let panel = Paragraph::new(self.lines).block(block);
        frame.render_widget(panel, area);
    }
}

struct MarkdownPanel<'a> {
    title: &'a str,
    border_color: Color,
    scroll: u16,
    text: Text<'a>,
}

impl<'a> MarkdownPanel<'a> {
    fn new(title: &'a str, border_color: Color, scroll: u16, text: Text<'a>) -> Self {
        Self {
            title,
            border_color,
            scroll,
            text,
        }
    }

    fn render(self, frame: &mut Frame, area: Rect) -> DetailViewport {
        let block = panel_block(self.title).border_style(Style::default().fg(self.border_color));
        let inner_width = block.inner(area).width;
        let total_lines = wrapped_line_count(&self.text.lines, inner_width);
        let content = Paragraph::new(self.text)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0))
            .block(block);

        frame.render_widget(content, area);
        let viewport_length = area.height.saturating_sub(2) as usize;
        if total_lines > viewport_length && area.height >= 3 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::default().fg(Color::Cyan))
                .track_style(Style::default().fg(Color::DarkGray));
            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(self.scroll as usize)
                .viewport_content_length(viewport_length);
            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
        DetailViewport::new(total_lines)
    }
}

struct MarkdownDetailPanel<'a> {
    title: &'a str,
    border_color: Color,
    scroll: u16,
    markdown: &'a str,
}

impl<'a> MarkdownDetailPanel<'a> {
    fn new(title: &'a str, border_color: Color, scroll: u16, markdown: &'a str) -> Self {
        Self {
            title,
            border_color,
            scroll,
            markdown,
        }
    }

    fn render(self, frame: &mut Frame, area: Rect) -> DetailViewport {
        let text = crate::terminal_md::render_to_tui_text(self.markdown);
        MarkdownPanel::new(self.title, self.border_color, self.scroll, text).render(frame, area)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DetailViewport {
    total_lines: usize,
}

impl DetailViewport {
    fn new(total_lines: usize) -> Self {
        Self { total_lines }
    }

    pub(super) fn footer_status(self, scroll: &mut u16) -> String {
        let max_scroll = self.total_lines.saturating_sub(1) as u16;
        if *scroll > max_scroll {
            *scroll = max_scroll;
        }
        format!(
            "Scroll {}/{}",
            (*scroll).saturating_add(1),
            self.total_lines
        )
    }
}

struct MetadataLine<'a> {
    label: &'static str,
    value: Vec<Span<'a>>,
}

impl<'a> MetadataLine<'a> {
    fn plain(label: &'static str, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label,
            value: vec![Span::raw(value.into())],
        }
    }

    fn styled(label: &'static str, value: impl Into<Cow<'a, str>>, style: Style) -> Self {
        Self {
            label,
            value: vec![Span::styled(value.into(), style)],
        }
    }

    fn status(label: &'static str, status: &str) -> Self {
        Self {
            label,
            value: StatusText::new(status).spans(),
        }
    }

    fn phase(label: &'static str, phase: &str) -> Self {
        Self {
            label,
            value: vec![Span::styled(phase.to_string(), phase_style(phase))],
        }
    }

    fn joined(label: &'static str, values: &[String], separator: &str) -> Self {
        Self::plain(label, values.join(separator))
    }

    fn tags(label: &'static str, tags: &[String]) -> Self {
        Self {
            label,
            value: vec![Span::styled(
                tags.join("  "),
                Style::default().fg(Color::Magenta).bold(),
            )],
        }
    }

    fn render(mut self) -> Line<'a> {
        let mut spans = vec![Span::styled(
            self.label,
            Style::default().fg(Color::DarkGray),
        )];
        spans.append(&mut self.value);
        Line::from(spans)
    }
}

pub(super) fn draw_rfc(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) {
    let Some(rfc) = app.index.rfcs.get(idx) else {
        return;
    };

    let status = rfc.rfc.status.as_ref();
    let phase = rfc.rfc.phase.as_ref();

    let mut header_lines = vec![
        MetadataLine::styled("ID:      ", &rfc.rfc.rfc_id, Style::default().bold()).render(),
        MetadataLine::plain("Title:   ", &rfc.rfc.title).render(),
        MetadataLine::styled(
            "Version: ",
            &rfc.rfc.version,
            Style::default().fg(Color::Cyan),
        )
        .render(),
        MetadataLine::status("Status:  ", status).render(),
        MetadataLine::phase("Phase:   ", phase).render(),
        MetadataLine::joined("Owners:  ", &rfc.rfc.owners, ", ").render(),
    ];

    if !rfc.rfc.refs.is_empty() {
        header_lines.push(MetadataLine::joined("Refs:    ", &rfc.rfc.refs, ", ").render());
    }

    if let Some(supersedes) = &rfc.rfc.supersedes {
        header_lines.push(MetadataLine::plain("Supersedes: ", supersedes.as_str()).render());
    }

    if !rfc.rfc.tags.is_empty() {
        header_lines.push(MetadataLine::tags("Tags:    ", &rfc.rfc.tags).render());
    }

    let suppress_clauses = rfc.rfc.status == RfcStatus::Deprecated;
    if suppress_clauses
        && let Some(replacement) = app
            .index
            .rfcs
            .iter()
            .find(|candidate| candidate.rfc.supersedes.as_deref() == Some(&rfc.rfc.rfc_id))
    {
        header_lines
            .push(MetadataLine::plain("Superseded by: ", replacement.rfc.rfc_id.as_str()).render());
    }

    let header_panel = MetadataPanel::new(
        format!("RFC // {}", rfc.rfc.rfc_id),
        Color::Blue,
        header_lines,
    );
    let header_height = header_panel.outer_height();

    if suppress_clauses {
        header_panel.render(frame, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(5)])
        .split(area);

    header_panel.render(frame, chunks[0]);

    let clause_items = rfc
        .clauses
        .iter()
        .map(|clause| {
            let clause_status = clause.spec.status.as_ref();
            ClauseListRow {
                id: &clause.spec.clause_id,
                title: &clause.spec.title,
                status: clause_status,
            }
            .render()
        })
        .collect();

    SelectableList::new("CLAUSE INDEX", Color::Cyan, clause_items).render(
        frame,
        chunks[1],
        &mut app.clause_list_state,
    );
}

pub(super) fn draw_adr(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) -> DetailViewport {
    let Some(adr) = app.index.adrs.get(idx) else {
        return DetailViewport::new(0);
    };

    let markdown = crate::render::render_adr_with_projection(adr, RenderProjection::Current)
        .unwrap_or_default();
    let title = format!("ADR // {}", adr.meta().id);
    MarkdownDetailPanel::new(&title, Color::Green, app.scroll, &markdown).render(frame, area)
}

pub(super) fn draw_work(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    idx: usize,
) -> DetailViewport {
    let Some(item) = app.index.work_items.get(idx) else {
        return DetailViewport::new(0);
    };

    let markdown = crate::render::render_work_item(item).unwrap_or_default();
    let title = format!("WI // {}", item.meta().id);
    MarkdownDetailPanel::new(&title, Color::Yellow, app.scroll, &markdown).render(frame, area)
}

// Implements [[RFC-0007:C-COCKPIT-VIEWS]]: guard artifacts are browsable read-only.
pub(super) fn draw_guard(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    idx: usize,
) -> DetailViewport {
    let Some(guard) = app.supplement.guards.get(idx) else {
        return DetailViewport::new(0);
    };
    let meta = guard.meta();
    let mut markdown = format!(
        "# {}\n\n**ID:** {}\n\n**Command:** `{}`\n\n**Timeout:** {} seconds\n",
        meta.title, meta.id, guard.spec.check.command, guard.spec.check.timeout_secs
    );
    if let Some(pattern) = &guard.spec.check.pattern {
        markdown.push_str(&format!("\n**Pattern:** `{pattern}`\n"));
    }
    if !meta.refs.is_empty() {
        markdown.push_str(&format!("\n**References:** {}\n", meta.refs.join(", ")));
    }
    if !meta.tags.is_empty() {
        markdown.push_str(&format!("\n**Tags:** `{}`\n", meta.tags.join("`, `")));
    }
    let title = format!("GUARD // {}", meta.id);
    MarkdownDetailPanel::new(&title, Color::LightBlue, app.scroll, &markdown).render(frame, area)
}

// Implements [[RFC-0007:C-LOOP-VIEWS]] and [[RFC-0007:C-LOOP-DAG]].
pub(super) fn draw_loop(frame: &mut Frame, app: &mut App, area: Rect, idx: usize) {
    let Some(entry) = app.supplement.loops.get(idx) else {
        return;
    };
    let Some(state) = entry.state.as_ref() else {
        let message = entry
            .diagnostic
            .as_ref()
            .map(|diag| diag.to_string())
            .unwrap_or_else(|| "Invalid loop state".to_string());
        frame.render_widget(
            Paragraph::new(message)
                .wrap(Wrap { trim: false })
                .block(panel_block("LOOP CONTROL").border_style(Style::default().fg(Color::Red))),
            area,
        );
        return;
    };

    let selected = app.selected_loop_work_id(idx);
    let chunks = if area.width >= 110 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area)
    };

    let max_lines = chunks[0].height.saturating_sub(2) as usize;
    let dag_items = match dag_lines(state, selected.as_deref(), max_lines) {
        Ok(lines) => lines
            .into_iter()
            .map(|line| {
                let style = if line.hidden {
                    Style::default().fg(Color::DarkGray)
                } else if line.selected {
                    Style::default().fg(Color::Cyan).bold()
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(line.text, style)))
            })
            .collect::<Vec<_>>(),
        Err(diagnostic) => vec![ListItem::new(Line::from(Span::styled(
            format!(
                "DAG unavailable for {} (selected: {}, max lines: {}): {}",
                state.loop_meta.id,
                selected.as_deref().unwrap_or("-"),
                max_lines,
                diagnostic
            ),
            Style::default().fg(Color::Red),
        )))],
    };
    let dag = List::new(dag_items)
        .block(panel_block("DEPENDENCY DAG").border_style(Style::default().fg(Color::Yellow)));
    frame.render_widget(dag, chunks[0]);

    let inspector = loop_inspector_lines(state, selected.as_deref());
    frame.render_widget(
        Paragraph::new(inspector)
            .wrap(Wrap { trim: false })
            .block(panel_block("SELECTED WORK").border_style(Style::default().fg(Color::Cyan))),
        chunks[1],
    );
}

// Implements [[RFC-0007:C-LOOP-VIEWS]]: selected loop work inspector.
fn loop_inspector_lines<'a>(
    state: &'a crate::loop_state::LoopState,
    selected: Option<&'a str>,
) -> Vec<Line<'a>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Loop: ", Style::default().fg(Color::DarkGray)),
            Span::raw(state.loop_meta.id.clone()),
        ]),
        Line::from(vec![
            Span::styled("State: ", Style::default().fg(Color::DarkGray)),
            Span::raw(state.loop_meta.state.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Next:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(state.loop_meta.next_action.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Work:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(state.loop_meta.work.join(", ")),
        ]),
        Line::from(""),
    ];

    let Some(work_id) = selected else {
        lines.push(Line::from("No work item selected"));
        return lines;
    };
    lines.push(Line::from(vec![
        Span::styled("ID:     ", Style::default().fg(Color::DarkGray)),
        Span::styled(work_id.to_string(), Style::default().bold()),
    ]));
    if let Some(item) = state.items.get(work_id) {
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::raw(item.status.as_str()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Rounds: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} (last {})", item.round_count, item.last_round)),
        ]));
    }
    let deps = state
        .dependencies
        .get(work_id)
        .filter(|deps| !deps.is_empty())
        .map(|deps| deps.join(", "))
        .unwrap_or_else(|| "-".to_string());
    lines.push(Line::from(vec![
        Span::styled("Needs:  ", Style::default().fg(Color::DarkGray)),
        Span::raw(deps),
    ]));
    let dependents = state
        .dependencies
        .iter()
        .filter_map(|(candidate, deps)| {
            deps.iter()
                .any(|dep| dep == work_id)
                .then_some(candidate.clone())
        })
        .collect::<Vec<_>>();
    lines.push(Line::from(vec![
        Span::styled("Feeds:  ", Style::default().fg(Color::DarkGray)),
        Span::raw(if dependents.is_empty() {
            "-".to_string()
        } else {
            dependents.join(", ")
        }),
    ]));
    lines
}

pub(super) fn draw_clause(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    rfc_idx: usize,
    clause_idx: usize,
) -> DetailViewport {
    let Some(rfc) = app.index.rfcs.get(rfc_idx) else {
        return DetailViewport::new(0);
    };

    let Some(clause) = rfc.clauses.get(clause_idx) else {
        return DetailViewport::new(0);
    };

    let mut raw = String::new();
    crate::render::render_clause_with_projection(
        &mut raw,
        &rfc.rfc.rfc_id,
        clause,
        RenderProjection::Current,
    );

    let title = format!("CLAUSE // {}", clause.spec.clause_id);
    MarkdownDetailPanel::new(&title, Color::Magenta, app.scroll, &raw).render(frame, area)
}

#[cfg(test)]
#[path = "detail_tests.rs"]
mod tests;
