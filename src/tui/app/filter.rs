use super::{App, View};

impl App {
    /// Get the total count of items in current list view (unfiltered)
    pub fn list_total_len(&self) -> usize {
        match self.view {
            View::RfcList => self.index.rfcs.len(),
            View::AdrList => self.index.adrs.len(),
            View::WorkList => self.index.work_items.len(),
            _ => 0,
        }
    }

    pub(super) fn invalidate_indices(&mut self) {
        self.indices_dirty = true;
    }

    fn recompute_indices(&mut self) {
        if !self.indices_dirty {
            return;
        }
        let query = self.filter_query.trim().to_ascii_lowercase();
        let has_query = !query.is_empty();
        self.cached_indices = match self.view {
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
        };
        self.indices_dirty = false;
    }

    /// Get indices for items in current list view (filtered, cached).
    pub fn list_indices(&mut self) -> Vec<usize> {
        self.recompute_indices();
        self.cached_indices.clone()
    }

    /// Get the count of items in current list view (filtered).
    pub fn list_len(&mut self) -> usize {
        self.recompute_indices();
        self.cached_indices.len()
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
        self.invalidate_indices();
        self.ensure_selection_in_bounds();
    }

    /// Append a character to the filter query
    pub fn push_filter_char(&mut self, ch: char) {
        self.filter_query.push(ch);
        self.invalidate_indices();
        self.ensure_selection_in_bounds();
    }

    /// Remove last character from filter query
    pub fn pop_filter_char(&mut self) {
        self.filter_query.pop();
        self.invalidate_indices();
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
}
