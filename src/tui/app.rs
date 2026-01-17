//! Application state for TUI.

use crate::model::ProjectIndex;

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
}

/// Application state
pub struct App {
    /// Project data
    pub index: ProjectIndex,
    /// Current view
    pub view: View,
    /// Selected index in list views
    pub selected: usize,
    /// Scroll offset for detail views
    pub scroll: u16,
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
            scroll: 0,
            should_quit: false,
        }
    }

    /// Get the count of items in current list view
    pub fn list_len(&self) -> usize {
        match self.view {
            View::RfcList => self.index.rfcs.len(),
            View::AdrList => self.index.adrs.len(),
            View::WorkList => self.index.work_items.len(),
            _ => 0,
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let len = self.list_len();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
        }
    }

    /// Enter detail view for selected item
    pub fn enter_detail(&mut self) {
        if self.list_len() == 0 {
            return;
        }
        self.view = match self.view {
            View::RfcList => View::RfcDetail(self.selected),
            View::AdrList => View::AdrDetail(self.selected),
            View::WorkList => View::WorkDetail(self.selected),
            _ => return,
        };
        self.scroll = 0;
    }

    /// Go back to previous view
    pub fn go_back(&mut self) {
        self.view = match self.view {
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
    }

    /// Navigate to a specific view
    pub fn go_to(&mut self, view: View) {
        self.view = view;
        self.selected = 0;
        self.scroll = 0;
    }

    /// Scroll down in detail view
    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    /// Scroll up in detail view
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}
