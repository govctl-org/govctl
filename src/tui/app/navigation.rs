use super::{App, View};
use ratatui::widgets::{ListState, TableState};

impl App {
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
            View::ClauseList => self
                .supplement
                .clauses
                .get(real_idx)
                .and_then(|entry| {
                    self.index
                        .rfcs
                        .iter()
                        .position(|rfc| rfc.rfc.rfc_id == entry.rfc_id)
                        .and_then(|rfc_idx| {
                            self.index.rfcs[rfc_idx]
                                .clauses
                                .iter()
                                .position(|clause| {
                                    clause.spec.clause_id == entry.clause.spec.clause_id
                                })
                                .map(|clause_idx| View::ClauseDetail(rfc_idx, clause_idx))
                        })
                })
                .unwrap_or(View::ClauseList),
            View::AdrList => View::AdrDetail(real_idx),
            View::WorkList => View::WorkDetail(real_idx),
            View::GuardList => View::GuardDetail(real_idx),
            View::Search => {
                self.enter_search_result_at(real_idx);
                return;
            }
            View::LoopList => {
                self.loop_selected = 0;
                View::LoopDetail(real_idx)
            }
            View::DiagnosticList => {
                self.enter_diagnostic_target_at(real_idx);
                return;
            }
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
            View::GuardDetail(_) => View::GuardList,
            View::LoopDetail(_) => View::LoopList,
            View::RfcList
            | View::ClauseList
            | View::AdrList
            | View::WorkList
            | View::GuardList
            | View::ReleaseList
            | View::TagList
            | View::Search
            | View::LoopList
            | View::DiagnosticList => View::Dashboard,
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
        self.invalidate_indices();
        if matches!(
            self.view,
            View::RfcList
                | View::ClauseList
                | View::AdrList
                | View::WorkList
                | View::GuardList
                | View::ReleaseList
                | View::TagList
                | View::Search
                | View::LoopList
                | View::DiagnosticList
        ) {
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

    /// Scroll down by half a page
    pub fn scroll_half_page_down(&mut self) {
        let half = (self.content_height / 2).max(1);
        self.scroll = self.scroll.saturating_add(half);
    }

    /// Scroll up by half a page
    pub fn scroll_half_page_up(&mut self) {
        let half = (self.content_height / 2).max(1);
        self.scroll = self.scroll.saturating_sub(half);
    }

    /// Scroll down by a full page
    pub fn scroll_page_down(&mut self) {
        let page = self.content_height.max(1);
        self.scroll = self.scroll.saturating_add(page);
    }

    /// Scroll up by a full page
    pub fn scroll_page_up(&mut self) {
        let page = self.content_height.max(1);
        self.scroll = self.scroll.saturating_sub(page);
    }

    /// Jump list selection by half a page
    pub fn select_half_page_down(&mut self) {
        let half = (self.content_height / 2).max(1) as usize;
        let len = self.list_len();
        if len > 0 {
            self.selected = (self.selected + half).min(len - 1);
            self.table_state.select(Some(self.selected));
        }
    }

    /// Jump list selection up by half a page
    pub fn select_half_page_up(&mut self) {
        let half = (self.content_height / 2).max(1) as usize;
        self.selected = self.selected.saturating_sub(half);
        self.table_state.select(Some(self.selected));
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

    pub fn loop_item_count(&self) -> usize {
        match self.view {
            View::LoopDetail(idx) => self.current_loop_order(idx).len(),
            _ => 0,
        }
    }

    pub fn loop_item_next(&mut self) {
        let len = self.loop_item_count();
        if len > 0 && self.loop_selected < len - 1 {
            self.loop_selected += 1;
        }
    }

    pub fn loop_item_prev(&mut self) {
        self.loop_selected = self.loop_selected.saturating_sub(1);
    }
}
