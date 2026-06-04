//! Application state for TUI.

use crate::model::ProjectIndex;
use ratatui::widgets::{ListState, TableState};

mod filter;
mod navigation;

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
    /// Visible content height (set during draw, used for page-scroll)
    pub content_height: u16,
    /// Active filter query for list views
    pub filter_query: String,
    /// Cached filtered indices (invalidated on filter/view change)
    cached_indices: Vec<usize>,
    indices_dirty: bool,
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
            content_height: 0,
            filter_query: String::new(),
            cached_indices: Vec::new(),
            indices_dirty: true,
            filter_mode: false,
            show_help: false,
            should_quit: false,
        }
    }
}
