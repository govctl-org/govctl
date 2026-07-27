use super::{App, View};
use crate::diagnostic::DiagnosticLevel;

fn fields_match_normalized_query(query: &str, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(query))
}

fn matching_indices<T>(
    items: &[T],
    has_query: bool,
    mut matches_query: impl FnMut(&T) -> bool,
) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (!has_query || matches_query(item)).then_some(index))
        .collect()
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
            View::ConformanceList => self.index.conformance_cases.len(),
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
            View::RfcList => matching_indices(&self.index.rfcs, has_query, |rfc| {
                fields_match_normalized_query(
                    &query,
                    &[
                        rfc.rfc.rfc_id.as_str(),
                        rfc.rfc.title.as_str(),
                        rfc.rfc.status.as_ref(),
                        rfc.rfc.phase.as_ref(),
                    ],
                )
            }),
            View::ClauseList => matching_indices(&self.supplement.clauses, has_query, |entry| {
                let clause = &entry.clause.spec;
                fields_match_normalized_query(
                    &query,
                    &[
                        entry.rfc_id.as_str(),
                        clause.clause_id.as_str(),
                        clause.title.as_str(),
                        clause.status.as_ref(),
                        clause.kind.as_ref(),
                    ],
                )
            }),
            View::AdrList => matching_indices(&self.index.adrs, has_query, |adr| {
                let meta = adr.meta();
                fields_match_normalized_query(
                    &query,
                    &[meta.id.as_str(), meta.title.as_str(), meta.status.as_ref()],
                )
            }),
            View::WorkList => matching_indices(&self.index.work_items, has_query, |item| {
                let meta = item.meta();
                fields_match_normalized_query(
                    &query,
                    &[meta.id.as_str(), meta.title.as_str(), meta.status.as_ref()],
                )
            }),
            View::GuardList => matching_indices(&self.supplement.guards, has_query, |guard| {
                let meta = guard.meta();
                fields_match_normalized_query(
                    &query,
                    &[
                        meta.id.as_str(),
                        meta.title.as_str(),
                        guard.spec.check.command.as_str(),
                    ],
                )
            }),
            View::ConformanceList => {
                matching_indices(&self.index.conformance_cases, has_query, |case| {
                    let trace = crate::model::derive_conformance_trace(case, &self.index);
                    let requirements = trace
                        .requirements
                        .iter()
                        .map(|binding| {
                            format!(
                                "{}@{}",
                                binding.clause_ref.as_str(),
                                binding.version.as_str()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let applicability = trace.requirement_applicability.to_string();
                    let guards = trace.guards.join(" ");
                    fields_match_normalized_query(
                        &query,
                        &[
                            trace.id.as_str(),
                            trace.title.as_str(),
                            trace.path.as_str(),
                            trace.selector.as_str(),
                            applicability.as_str(),
                            requirements.as_str(),
                            guards.as_str(),
                        ],
                    )
                })
            }
            View::ReleaseList => {
                matching_indices(&self.supplement.releases, has_query, |release| {
                    fields_match_normalized_query(
                        &query,
                        &[release.version.as_str(), release.date.as_str()],
                    )
                })
            }
            View::TagList => matching_indices(&self.supplement.tags, has_query, |tag| {
                fields_match_normalized_query(&query, &[tag.name.as_str()])
            }),
            View::Search => matching_indices(&self.search_results, has_query, |result| {
                fields_match_normalized_query(
                    &query,
                    &[
                        result.kind.as_str(),
                        result.id.as_str(),
                        result.title.as_str(),
                        result.snippet.as_str(),
                    ],
                )
            }),
            View::LoopList => matching_indices(&self.supplement.loops, has_query, |entry| {
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
                fields_match_normalized_query(&query, &[entry.id.as_str(), state, work.as_str()])
            }),
            View::DiagnosticList => {
                matching_indices(&self.supplement.diagnostics, has_query, |diagnostic| {
                    fields_match_normalized_query(
                        &query,
                        &[
                            diagnostic.code.code(),
                            diagnostic_level_label(diagnostic.level),
                            diagnostic.message.as_str(),
                            diagnostic.file.as_str(),
                        ],
                    )
                })
            }
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
