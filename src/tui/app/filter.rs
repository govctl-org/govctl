use super::{App, View};
use crate::diagnostic::DiagnosticLevel;

fn fields_match_normalized_query(query: &str, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(query))
}

impl App {
    /// Get the total count of items in current list view (unfiltered)
    pub fn list_total_len(&self) -> usize {
        match self.view {
            View::RfcList => self.index.rfcs.len(),
            View::ClauseList => self.supplement.clauses.len(),
            View::AdrList => self.index.adrs.len(),
            View::WorkList => self.index.work_items.len(),
            View::GuardList => self.supplement.guards.len(),
            View::ReleaseList => self.supplement.releases.len(),
            View::TagList => self.supplement.tags.len(),
            View::Search => self.search_results.len(),
            View::LoopList => self.supplement.loops.len(),
            View::DiagnosticList => self.supplement.diagnostics.len(),
            _ => 0,
        }
    }

    // Implements [[RFC-0007:C-COCKPIT-VIEWS]]: cached list indices follow view data.
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
                    if fields_match_normalized_query(
                        &query,
                        &[
                            rfc.rfc.rfc_id.as_str(),
                            rfc.rfc.title.as_str(),
                            rfc.rfc.status.as_ref(),
                            rfc.rfc.phase.as_ref(),
                        ],
                    ) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::ClauseList => self
                .supplement
                .clauses
                .iter()
                .enumerate()
                .filter_map(|(idx, entry)| {
                    if !has_query {
                        return Some(idx);
                    }
                    let clause = &entry.clause.spec;
                    if fields_match_normalized_query(
                        &query,
                        &[
                            entry.rfc_id.as_str(),
                            clause.clause_id.as_str(),
                            clause.title.as_str(),
                            clause.status.as_ref(),
                            clause.kind.as_ref(),
                        ],
                    ) {
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
                    if fields_match_normalized_query(
                        &query,
                        &[meta.id.as_str(), meta.title.as_str(), meta.status.as_ref()],
                    ) {
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
                    if fields_match_normalized_query(
                        &query,
                        &[meta.id.as_str(), meta.title.as_str(), meta.status.as_ref()],
                    ) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::GuardList => self
                .supplement
                .guards
                .iter()
                .enumerate()
                .filter_map(|(idx, guard)| {
                    if !has_query {
                        return Some(idx);
                    }
                    let meta = guard.meta();
                    if fields_match_normalized_query(
                        &query,
                        &[
                            meta.id.as_str(),
                            meta.title.as_str(),
                            guard.spec.check.command.as_str(),
                        ],
                    ) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::ReleaseList => self
                .supplement
                .releases
                .iter()
                .enumerate()
                .filter_map(|(idx, release)| {
                    if !has_query
                        || fields_match_normalized_query(
                            &query,
                            &[release.version.as_str(), release.date.as_str()],
                        )
                    {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::TagList => self
                .supplement
                .tags
                .iter()
                .enumerate()
                .filter_map(|(idx, tag)| {
                    if !has_query || fields_match_normalized_query(&query, &[tag.name.as_str()]) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::Search => self
                .search_results
                .iter()
                .enumerate()
                .filter_map(|(idx, result)| {
                    if !has_query
                        || fields_match_normalized_query(
                            &query,
                            &[
                                result.kind.as_str(),
                                result.id.as_str(),
                                result.title.as_str(),
                                result.snippet.as_str(),
                            ],
                        )
                    {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::LoopList => self
                .supplement
                .loops
                .iter()
                .enumerate()
                .filter_map(|(idx, entry)| {
                    if !has_query {
                        return Some(idx);
                    }
                    let state = entry
                        .state
                        .as_ref()
                        .map(|state| state.loop_meta.state.as_str())
                        .unwrap_or("invalid");
                    let work = entry
                        .state
                        .as_ref()
                        .map(|state| state.loop_meta.work.join(" "))
                        .unwrap_or_default();
                    if fields_match_normalized_query(
                        &query,
                        &[entry.id.as_str(), state, work.as_str()],
                    ) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect(),
            View::DiagnosticList => self
                .supplement
                .diagnostics
                .iter()
                .enumerate()
                .filter_map(|(idx, diagnostic)| {
                    if !has_query
                        || fields_match_normalized_query(
                            &query,
                            &[
                                diagnostic.code.code(),
                                diagnostic_level_label(diagnostic.level),
                                diagnostic.message.as_str(),
                                diagnostic.file.as_str(),
                            ],
                        )
                    {
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

fn diagnostic_level_label(level: DiagnosticLevel) -> &'static str {
    match level {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info => "info",
    }
}

#[cfg(test)]
mod tests {
    use super::fields_match_normalized_query;

    #[test]
    fn fields_match_normalized_query_checks_mixed_case_fields() {
        assert!(fields_match_normalized_query(
            "norm",
            &["RFC-0001", "Title", "Normative", "Spec"],
        ));
        assert!(fields_match_normalized_query(
            "rfc-0001",
            &["RFC-0001", "Title", "draft"],
        ));
        assert!(!fields_match_normalized_query(
            "missing",
            &["RFC-0001", "Title", "draft"],
        ));
    }
}
