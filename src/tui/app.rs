//! Application state for TUI.

use crate::model::ProjectIndex;
use ratatui::widgets::{ListState, TableState};

/// Current view in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Dashboard,
    RfcList,
    AdrList,
    WorkList,
    RfcDetail(usize),
    AdrDetail(usize),
    WorkDetail(usize),
    /// Clause detail view: (rfc_index, clause_index)
    ClauseDetail(usize, usize),
}

/// Application state
pub struct App {
    /// Project data
    pub index: ProjectIndex,
    /// Current view
    pub view: View,
    /// Selected index in list views
    pub selected: usize,
    /// Table state for scrollable list views
    pub table_state: TableState,
    /// List state for clause selection in RFC detail view
    pub clause_list_state: ListState,
    /// Scroll offset for detail views
    pub scroll: u16,
    /// Active filter query for list views
    pub filter_query: String,
    /// Whether filter input mode is active
    pub filter_mode: bool,
    /// Show help overlay
    pub show_help: bool,
    /// Should quit
    pub should_quit: bool,
}

impl App {
    /// Create new app with loaded project index
    pub fn new(mut index: ProjectIndex) -> Self {
        // Sort all items by ID for consistent display
        index.rfcs.sort_by(|a, b| a.rfc.rfc_id.cmp(&b.rfc.rfc_id));
        index.adrs.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));
        index
            .work_items
            .sort_by(|a, b| a.meta().id.cmp(&b.meta().id));

        Self {
            index,
            view: View::Dashboard,
            selected: 0,
            table_state: TableState::default().with_selected(Some(0)),
            clause_list_state: ListState::default().with_selected(Some(0)),
            scroll: 0,
            filter_query: String::new(),
            filter_mode: false,
            show_help: false,
            should_quit: false,
        }
    }

    /// Get the total count of items in current list view (unfiltered)
    pub fn list_total_len(&self) -> usize {
        match self.view {
            View::RfcList => self.index.rfcs.len(),
            View::AdrList => self.index.adrs.len(),
            View::WorkList => self.index.work_items.len(),
            _ => 0,
        }
    }

    /// Get indices for items in current list view (filtered)
    pub fn list_indices(&self) -> Vec<usize> {
        let query = self.filter_query.trim().to_ascii_lowercase();
        let has_query = !query.is_empty();
        match self.view {
            View::RfcList => self
                .index
                .rfcs
                .iter()
                .enumerate()
                .filter_map(|(idx, rfc)| {
                    if !has_query {
                        return Some(idx);
                    }
                    let status = rfc.rfc.status.as_ref().to_ascii_lowercase();
                    let phase = rfc.rfc.phase.as_ref().to_ascii_lowercase();
                    let id = rfc.rfc.rfc_id.to_ascii_lowercase();
                    let title = rfc.rfc.title.to_ascii_lowercase();
                    if id.contains(&query)
                        || title.contains(&query)
                        || status.contains(&query)
                        || phase.contains(&query)
                    {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::AdrList => self
                .index
                .adrs
                .iter()
                .enumerate()
                .filter_map(|(idx, adr)| {
                    if !has_query {
                        return Some(idx);
                    }
                    let meta = adr.meta();
                    let status = meta.status.as_ref().to_ascii_lowercase();
                    let id = meta.id.to_ascii_lowercase();
                    let title = meta.title.to_ascii_lowercase();
                    if id.contains(&query) || title.contains(&query) || status.contains(&query) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::WorkList => self
                .index
                .work_items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    if !has_query {
                        return Some(idx);
                    }
                    let meta = item.meta();
                    let status = meta.status.as_ref().to_ascii_lowercase();
                    let id = meta.id.to_ascii_lowercase();
                    let title = meta.title.to_ascii_lowercase();
                    if id.contains(&query) || title.contains(&query) || status.contains(&query) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Get the count of items in current list view (filtered)
    pub fn list_len(&self) -> usize {
        self.list_indices().len()
    }

    /// Whether a list filter is active
    pub fn filter_active(&self) -> bool {
        !self.filter_query.trim().is_empty()
    }

    /// Enter filter input mode
    pub fn enter_filter_mode(&mut self) {
        self.filter_mode = true;
    }

    /// Exit filter input mode
    pub fn exit_filter_mode(&mut self) {
        self.filter_mode = false;
    }

    /// Clear filter query
    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.ensure_selection_in_bounds();
    }

    /// Append a character to the filter query
    pub fn push_filter_char(&mut self, ch: char) {
        self.filter_query.push(ch);
        self.ensure_selection_in_bounds();
    }

    /// Remove last character from filter query
    pub fn pop_filter_char(&mut self) {
        self.filter_query.pop();
        self.ensure_selection_in_bounds();
    }

    /// Ensure selected index is valid for current list
    pub fn ensure_selection_in_bounds(&mut self) {
        let len = self.list_len();
        if len == 0 {
            self.selected = 0;
            self.table_state.select(None);
            return;
        }
        if self.selected >= len {
            self.selected = len - 1;
        }
        self.table_state.select(Some(self.selected));
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.table_state.select(Some(self.selected));
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let len = self.list_len();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
        }
        self.table_state.select(Some(self.selected));
    }

    /// Jump to first item in list
    pub fn select_top(&mut self) {
        if self.list_len() == 0 {
            self.table_state.select(None);
            return;
        }
        self.selected = 0;
        self.table_state.select(Some(self.selected));
    }

    /// Jump to last item in list
    pub fn select_bottom(&mut self) {
        let len = self.list_len();
        if len == 0 {
            self.table_state.select(None);
            return;
        }
        self.selected = len - 1;
        self.table_state.select(Some(self.selected));
    }

    /// Enter detail view for selected item
    pub fn enter_detail(&mut self) {
        let indices = self.list_indices();
        if indices.is_empty() {
            return;
        }
        if self.selected >= indices.len() {
            self.ensure_selection_in_bounds();
            return;
        }
        let real_idx = indices[self.selected];
        self.view = match self.view {
            View::RfcList => {
                self.clause_list_state = ListState::default().with_selected(Some(0));
                View::RfcDetail(real_idx)
            }
            View::AdrList => View::AdrDetail(real_idx),
            View::WorkList => View::WorkDetail(real_idx),
            _ => return,
        };
        self.scroll = 0;
    }

    /// Go back to previous view
    pub fn go_back(&mut self) {
        self.view = match self.view {
            View::ClauseDetail(rfc_idx, _) => View::RfcDetail(rfc_idx),
            View::RfcDetail(_) => View::RfcList,
            View::AdrDetail(_) => View::AdrList,
            View::WorkDetail(_) => View::WorkList,
            View::RfcList | View::AdrList | View::WorkList => View::Dashboard,
            View::Dashboard => {
                self.should_quit = true;
                View::Dashboard
            }
        };
        self.scroll = 0;
        if self.view == View::Dashboard {
            self.filter_mode = false;
            self.clear_filter();
        }
    }

    /// Navigate to a specific view
    pub fn go_to(&mut self, view: View) {
        self.view = view;
        self.selected = 0;
        self.table_state = TableState::default().with_selected(Some(0));
        self.scroll = 0;
        if matches!(self.view, View::RfcList | View::AdrList | View::WorkList) {
            self.filter_mode = false;
            self.clear_filter();
        } else {
            self.filter_mode = false;
        }
    }

    /// Scroll down in detail view
    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    /// Scroll up in detail view
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    /// Get clause count for current RFC detail view
    pub fn clause_count(&self) -> usize {
        match self.view {
            View::RfcDetail(idx) => self
                .index
                .rfcs
                .get(idx)
                .map(|r| r.clauses.len())
                .unwrap_or(0),
            _ => 0,
        }
    }

    /// Move clause selection up
    pub fn clause_prev(&mut self) {
        let selected = self.clause_list_state.selected().unwrap_or(0);
        if selected > 0 {
            self.clause_list_state.select(Some(selected - 1));
        }
    }

    /// Move clause selection down
    pub fn clause_next(&mut self) {
        let len = self.clause_count();
        let selected = self.clause_list_state.selected().unwrap_or(0);
        if len > 0 && selected < len - 1 {
            self.clause_list_state.select(Some(selected + 1));
        }
    }

    /// Enter clause detail view from RFC detail
    pub fn enter_clause_detail(&mut self) {
        if let View::RfcDetail(rfc_idx) = self.view {
            let clause_idx = self.clause_list_state.selected().unwrap_or(0);
            if self.clause_count() > 0 {
                self.view = View::ClauseDetail(rfc_idx, clause_idx);
                self.scroll = 0;
            }
        }
    }
}
