//! UI rendering for TUI.

mod chrome;
mod components;
mod dashboard;
mod detail;
mod help;
mod lists;
#[cfg(test)]
mod test_support;

use super::app::{App, View};
use crate::theme::{phase_semantic, status_semantic};
use detail::DetailViewport;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Borders},
};

fn status_style(status: &str) -> Style {
    Style::default().fg(status_semantic(status).to_ratatui())
}

fn phase_style(phase: &str) -> Style {
    Style::default().fg(phase_semantic(phase).to_ratatui())
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

    chrome::Header::new(app).render(frame, chunks[0]);
    app.content_height = chunks[1].height;

    let mut footer_status = None;

    if let Some(viewport) = draw_content(frame, app, chunks[1]) {
        footer_status = Some(viewport.footer_status(&mut app.scroll));
    }

    chrome::Footer::new(app.view, footer_status.as_deref()).render(frame, chunks[2]);

    if app.show_help {
        help::draw_overlay(frame, app);
    }
}

fn draw_content(frame: &mut Frame, app: &mut App, area: Rect) -> Option<DetailViewport> {
    match app.view {
        View::Dashboard => {
            dashboard::draw(frame, app, area);
            None
        }
        View::RfcList => {
            lists::draw_rfc(frame, app, area);
            None
        }
        View::ClauseList => {
            lists::draw_clause(frame, app, area);
            None
        }
        View::AdrList => {
            lists::draw_adr(frame, app, area);
            None
        }
        View::WorkList => {
            lists::draw_work(frame, app, area);
            None
        }
        View::GuardList => {
            lists::draw_guard(frame, app, area);
            None
        }
        View::ReleaseList => {
            lists::draw_release(frame, app, area);
            None
        }
        View::TagList => {
            lists::draw_tag(frame, app, area);
            None
        }
        View::Search => {
            lists::draw_search(frame, app, area);
            None
        }
        View::LoopList => {
            lists::draw_loop(frame, app, area);
            None
        }
        View::LoopDetail(idx) => {
            detail::draw_loop(frame, app, area, idx);
            None
        }
        View::DiagnosticList => {
            lists::draw_diagnostics(frame, app, area);
            None
        }
        View::RfcDetail(idx) => {
            detail::draw_rfc(frame, app, area, idx);
            None
        }
        View::AdrDetail(idx) => {
            // Implements [[RFC-0003:C-DETAIL]]
            Some(detail::draw_adr(frame, app, area, idx))
        }
        View::WorkDetail(idx) => {
            // Implements [[RFC-0003:C-DETAIL]]
            Some(detail::draw_work(frame, app, area, idx))
        }
        View::GuardDetail(idx) => Some(detail::draw_guard(frame, app, area, idx)),
        View::ClauseDetail(rfc_idx, clause_idx) => {
            // Implements [[RFC-0003:C-DETAIL]]
            Some(detail::draw_clause(frame, app, area, rfc_idx, clause_idx))
        }
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

#[cfg(test)]
mod tests;
