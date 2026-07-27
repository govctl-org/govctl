use super::super::{panel_block, phase_style, status_style};
use crate::theme::status_icon;
use ratatui::{
    prelude::*,
    widgets::{
        HighlightSpacing, List, ListItem, ListState, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};

pub(in crate::tui::ui) struct ResourceTable {
    rows: Vec<Row<'static>>,
    spec: ResourceTableSpec,
}

pub(in crate::tui::ui) struct ResourceTableSpec {
    pub(in crate::tui::ui) widths: Vec<Constraint>,
    pub(in crate::tui::ui) compact_widths: Vec<Constraint>,
    pub(in crate::tui::ui) headers: &'static [&'static str],
    pub(in crate::tui::ui) header_color: Color,
    pub(in crate::tui::ui) title: &'static str,
    pub(in crate::tui::ui) border_color: Color,
}

impl ResourceTable {
    fn new(rows: Vec<Row<'static>>, spec: ResourceTableSpec) -> Self {
        Self { rows, spec }
    }

    pub(in crate::tui::ui) fn from_indexed_items<T>(
        items: &[T],
        indices: &[usize],
        spec: ResourceTableSpec,
        row_for: impl FnMut(&T) -> Row<'static>,
    ) -> Self {
        let rows = indices
            .iter()
            .filter_map(|&idx| items.get(idx))
            .map(row_for)
            .collect();

        Self::new(rows, spec)
    }

    pub(in crate::tui::ui) fn render(self, frame: &mut Frame, area: Rect, state: &mut TableState) {
        let row_count = self.rows.len();
        let selected = state.selected().unwrap_or(0);
        let widths = if area.width < 90 {
            self.spec.compact_widths
        } else {
            self.spec.widths
        };
        let block = panel_block(self.spec.title)
            .title_bottom(
                Line::from(record_count_label(row_count))
                    .right_aligned()
                    .style(Style::default().fg(Color::DarkGray)),
            )
            .border_style(Style::default().fg(self.spec.border_color));
        let table = Table::new(self.rows, widths)
            .header(
                Row::new(self.spec.headers.to_vec())
                    .style(Style::default().bold().fg(self.spec.header_color))
                    .bottom_margin(1),
            )
            .row_highlight_style(Style::default().fg(Color::Cyan).bold())
            .highlight_symbol("▌ ")
            .highlight_spacing(HighlightSpacing::Always)
            .column_spacing(2)
            .block(block);
        frame.render_stateful_widget(table, area, state);
        render_scrollbar(
            frame,
            area,
            row_count,
            selected,
            area.height.saturating_sub(4),
        );
    }
}

pub(in crate::tui::ui) struct ResourceListRow<'a> {
    pub(in crate::tui::ui) id: &'a str,
    pub(in crate::tui::ui) title: &'a str,
    pub(in crate::tui::ui) status: &'a str,
    pub(in crate::tui::ui) tags: &'a [String],
}

impl ResourceListRow<'_> {
    pub(in crate::tui::ui) fn render(&self) -> Row<'static> {
        Row::new(vec![
            Line::from(self.id.to_string()),
            Line::from(self.title.to_string()),
            StatusText::new(self.status).render(),
            TagsCell::new(self.tags).render(),
        ])
    }
}

pub(in crate::tui::ui) struct ClauseListRow<'a> {
    pub(in crate::tui::ui) id: &'a str,
    pub(in crate::tui::ui) title: &'a str,
    pub(in crate::tui::ui) status: &'a str,
}

impl ClauseListRow<'_> {
    pub(in crate::tui::ui) fn render(&self) -> ListItem<'static> {
        ListItem::new(Line::from(vec![
            StatusText::new(self.status).icon_span(),
            Span::styled(self.id.to_string(), Style::default().fg(Color::Blue).bold()),
            Span::raw(" — "),
            Span::raw(self.title.to_string()),
        ]))
    }
}

pub(in crate::tui::ui) struct StatusText<'a> {
    status: &'a str,
}

impl<'a> StatusText<'a> {
    pub(in crate::tui::ui) fn new(status: &'a str) -> Self {
        Self { status }
    }

    pub(in crate::tui::ui) fn render(&self) -> Line<'static> {
        Line::from(self.spans())
    }

    pub(in crate::tui::ui) fn spans<'b>(&self) -> Vec<Span<'b>> {
        vec![
            self.icon_span(),
            Span::styled(self.status.to_string(), status_style(self.status)),
        ]
    }

    pub(in crate::tui::ui) fn icon_span<'b>(&self) -> Span<'b> {
        Span::styled(
            format!("{} ", status_icon(self.status)),
            status_style(self.status),
        )
    }
}

pub(in crate::tui::ui) struct PhaseCell<'a> {
    phase: &'a str,
}

impl<'a> PhaseCell<'a> {
    pub(in crate::tui::ui) fn new(phase: &'a str) -> Self {
        Self { phase }
    }

    pub(in crate::tui::ui) fn render(&self) -> Line<'static> {
        Line::from(Span::styled(
            self.phase.to_string(),
            phase_style(self.phase),
        ))
    }
}

pub(in crate::tui::ui) struct TagsCell<'a> {
    tags: &'a [String],
}

impl<'a> TagsCell<'a> {
    pub(in crate::tui::ui) fn new(tags: &'a [String]) -> Self {
        Self { tags }
    }

    pub(in crate::tui::ui) fn render(&self) -> Line<'static> {
        Line::from(Span::styled(
            self.tags.join(" "),
            Style::default().fg(Color::Magenta),
        ))
    }
}

pub(in crate::tui::ui) struct SelectableList {
    title: String,
    border_color: Color,
    items: Vec<ListItem<'static>>,
}

impl SelectableList {
    pub(in crate::tui::ui) fn new(
        title: impl Into<String>,
        border_color: Color,
        items: Vec<ListItem<'static>>,
    ) -> Self {
        Self {
            title: title.into(),
            border_color,
            items,
        }
    }

    pub(in crate::tui::ui) fn render(self, frame: &mut Frame, area: Rect, state: &mut ListState) {
        let item_count = self.items.len();
        let selected = state.selected().unwrap_or(0);
        let list = List::new(self.items)
            .block(
                panel_block(&self.title)
                    .title_bottom(
                        Line::from(record_count_label(item_count))
                            .right_aligned()
                            .style(Style::default().fg(Color::DarkGray)),
                    )
                    .border_style(Style::default().fg(self.border_color)),
            )
            .highlight_style(Style::default().fg(Color::Cyan).bold())
            .highlight_symbol("▌ ")
            .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(list, area, state);
        render_scrollbar(
            frame,
            area,
            item_count,
            selected,
            area.height.saturating_sub(2),
        );
    }
}

fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    content_length: usize,
    position: usize,
    viewport_length: u16,
) {
    if content_length == 0 || content_length <= viewport_length as usize || area.height < 3 {
        return;
    }
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .thumb_style(Style::default().fg(Color::Cyan))
        .track_style(Style::default().fg(Color::DarkGray));
    let mut scrollbar_state = ScrollbarState::new(content_length)
        .position(position)
        .viewport_content_length(viewport_length as usize);
    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

fn record_count_label(count: usize) -> String {
    let noun = if count == 1 { "RECORD" } else { "RECORDS" };
    format!(" {count} {noun} ")
}
