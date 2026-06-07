//! Application state for TUI.

use super::data::{TuiLoopEntry, TuiSupplement, load_supplement};
use crate::cmd::search::SearchResult;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
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
    ClauseList,
    AdrList,
    WorkList,
    GuardList,
    ReleaseList,
    TagList,
    Search,
    LoopList,
    LoopDetail(usize),
    DiagnosticList,
    RfcDetail(usize),
    AdrDetail(usize),
    WorkDetail(usize),
    GuardDetail(usize),
    /// Clause detail view: (rfc_index, clause_index)
    ClauseDetail(usize, usize),
}

/// Application state
pub struct App {
    /// Project configuration
    pub config: Config,
    /// Project data
    pub index: ProjectIndex,
    /// Additional read-only cockpit data
    pub supplement: TuiSupplement,
    /// Current view
    pub view: View,
    /// Selected index in list views
    pub selected: usize,
    /// Selected work item index in loop detail views
    pub loop_selected: usize,
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
    /// Search query for the TUI search view
    pub search_query: String,
    /// Whether search input mode is active
    pub search_mode: bool,
    /// Search results from the last submitted query
    pub search_results: Vec<SearchResult>,
    /// Search error from the last submitted query
    pub search_error: Option<Diagnostic>,
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
            config: Config::default(),
            index,
            supplement: TuiSupplement::default(),
            view: View::Dashboard,
            selected: 0,
            loop_selected: 0,
            table_state: TableState::default().with_selected(Some(0)),
            clause_list_state: ListState::default().with_selected(Some(0)),
            scroll: 0,
            content_height: 0,
            filter_query: String::new(),
            cached_indices: Vec::new(),
            indices_dirty: true,
            search_query: String::new(),
            search_mode: false,
            search_results: Vec::new(),
            search_error: None,
            filter_mode: false,
            show_help: false,
            should_quit: false,
        }
    }

    pub fn with_project(config: Config, index: ProjectIndex) -> Self {
        let mut app = Self::new(index);
        let supplement = load_supplement(&config, &app.index);
        app.config = config;
        app.supplement = supplement;
        app
    }

    pub fn loop_entries(&self) -> &[TuiLoopEntry] {
        &self.supplement.loops
    }

    pub fn current_loop_state(&self, loop_idx: usize) -> Option<&crate::loop_state::LoopState> {
        self.supplement
            .loops
            .get(loop_idx)
            .and_then(|entry| entry.state.as_ref())
    }

    pub fn current_loop_order(&self, loop_idx: usize) -> Vec<String> {
        self.current_loop_state(loop_idx)
            .and_then(|state| crate::loop_planner::topological_order_for_state(state).ok())
            .unwrap_or_default()
    }

    pub fn selected_loop_work_id(&self, loop_idx: usize) -> Option<String> {
        self.current_loop_order(loop_idx)
            .get(self.loop_selected)
            .cloned()
    }

    // Implements [[RFC-0007:C-SEARCH]] and [[RFC-0007:C-READ-ONLY]].
    pub fn submit_search(&mut self) {
        let query = self.search_query.trim();
        if query.is_empty() {
            self.search_results.clear();
            self.search_error = None;
            self.selected = 0;
            self.table_state.select(None);
            self.invalidate_indices();
            return;
        }

        match crate::cmd::search::search_results(
            &self.config,
            &[query.to_string()],
            &[],
            &[],
            Some(50),
            false,
        ) {
            Ok(results) => {
                self.search_results = results;
                self.search_error = None;
            }
            Err(diagnostic) => {
                self.search_results.clear();
                self.search_error = Some(diagnostic);
            }
        }
        self.selected = 0;
        self.table_state = TableState::default()
            .with_selected((!self.search_results.is_empty()).then_some(self.selected));
        self.invalidate_indices();
    }

    pub fn push_search_char(&mut self, ch: char) {
        self.search_query.push(ch);
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
    }

    pub fn enter_search_result_at(&mut self, result_idx: usize) {
        let Some(result) = self.search_results.get(result_idx).cloned() else {
            return;
        };
        self.view = match result.kind.as_str() {
            "rfc" => self
                .index
                .rfcs
                .iter()
                .position(|rfc| rfc.rfc.rfc_id == result.id)
                .map(View::RfcDetail)
                .unwrap_or(View::Search),
            "clause" => self
                .index
                .rfcs
                .iter()
                .enumerate()
                .find_map(|(rfc_idx, rfc)| {
                    rfc.clauses
                        .iter()
                        .position(|clause| {
                            format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id) == result.id
                        })
                        .map(|clause_idx| View::ClauseDetail(rfc_idx, clause_idx))
                })
                .unwrap_or(View::Search),
            "adr" => self
                .index
                .adrs
                .iter()
                .position(|adr| adr.meta().id == result.id)
                .map(View::AdrDetail)
                .unwrap_or(View::Search),
            "work" => self
                .index
                .work_items
                .iter()
                .position(|item| item.meta().id == result.id)
                .map(View::WorkDetail)
                .unwrap_or(View::Search),
            "guard" => self
                .supplement
                .guards
                .iter()
                .position(|guard| guard.meta().id == result.id)
                .map(View::GuardDetail)
                .unwrap_or(View::Search),
            _ => View::Search,
        };
        self.scroll = 0;
    }

    pub fn enter_diagnostic_target_at(&mut self, diagnostic_idx: usize) {
        let Some(diagnostic) = self.supplement.diagnostics.get(diagnostic_idx) else {
            return;
        };
        let file = diagnostic.file.as_str();
        if let Some((idx, _)) = self
            .index
            .rfcs
            .iter()
            .enumerate()
            .find(|(_, rfc)| file == self.config.display_path(&rfc.path).display().to_string())
        {
            self.view = View::RfcDetail(idx);
            return;
        }
        if let Some((rfc_idx, clause_idx)) =
            self.index
                .rfcs
                .iter()
                .enumerate()
                .find_map(|(rfc_idx, rfc)| {
                    rfc.clauses
                        .iter()
                        .enumerate()
                        .find(|(_, clause)| {
                            file == self.config.display_path(&clause.path).display().to_string()
                        })
                        .map(|(clause_idx, _)| (rfc_idx, clause_idx))
                })
        {
            self.view = View::ClauseDetail(rfc_idx, clause_idx);
            return;
        }
        if let Some((idx, _)) = self
            .index
            .adrs
            .iter()
            .enumerate()
            .find(|(_, adr)| file == self.config.display_path(&adr.path).display().to_string())
        {
            self.view = View::AdrDetail(idx);
            return;
        }
        if let Some((idx, _)) =
            self.index.work_items.iter().enumerate().find(|(_, item)| {
                file == self.config.display_path(&item.path).display().to_string()
            })
        {
            self.view = View::WorkDetail(idx);
            return;
        }
        if let Some((idx, _)) = self
            .supplement
            .guards
            .iter()
            .enumerate()
            .find(|(_, guard)| file == self.config.display_path(&guard.path).display().to_string())
        {
            self.view = View::GuardDetail(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::loop_state::LoopState;
    use crate::model::{
        ClauseEntry, ClauseKind, ClauseSpec, ClauseStatus, GuardCheck, GuardEntry, GuardMeta,
        GuardSpec, ProjectIndex, RfcIndex, RfcPhase, RfcSpec, RfcStatus, WorkItemContent,
        WorkItemEntry, WorkItemMeta, WorkItemSpec, WorkItemStatus, WorkItemVerification,
    };
    use crate::tui::data::TuiLoopEntry;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn with_project_loads_supplement_from_sorted_index() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::TempDir::new()?;
        let config = Config {
            gov_root: temp_dir.path().join("gov"),
            ..Default::default()
        };
        let index = ProjectIndex {
            rfcs: vec![
                rfc_entry("RFC-0002", "C-TWO"),
                rfc_entry("RFC-0001", "C-ONE"),
            ],
            adrs: vec![],
            work_items: vec![],
        };

        let app = App::with_project(config, index);

        assert_eq!(app.index.rfcs[0].rfc.rfc_id, "RFC-0001");
        assert_eq!(
            app.supplement
                .clauses
                .iter()
                .map(|entry| entry.rfc_id.as_str())
                .collect::<Vec<_>>(),
            vec!["RFC-0001", "RFC-0002"]
        );
        Ok(())
    }

    #[test]
    fn empty_search_submit_invalidates_cached_indices() {
        let mut app = App::new(project_index());
        app.view = View::Search;
        app.search_results.push(SearchResult {
            kind: "rfc".to_string(),
            id: "RFC-0001".to_string(),
            title: "TUI cockpit".to_string(),
            path: "gov/rfc/RFC-0001/rfc.toml".to_string(),
            snippet: "TUI cockpit".to_string(),
            score: None,
            status: Some("normative".to_string()),
        });
        app.search_results.push(SearchResult {
            kind: "work".to_string(),
            id: "WI-2026-06-06-001".to_string(),
            title: "Implement TUI v2".to_string(),
            path: "gov/work/WI-2026-06-06-001.toml".to_string(),
            snippet: "work item".to_string(),
            score: None,
            status: Some("active".to_string()),
        });
        assert_eq!(app.list_indices(), vec![0, 1]);

        app.search_query = "   ".to_string();
        app.submit_search();

        assert!(app.search_results.is_empty());
        assert!(app.list_indices().is_empty());
    }

    #[test]
    fn search_result_enters_matching_detail_view() {
        let mut app = App::new(project_index());
        app.view = View::Search;
        app.search_results.push(SearchResult {
            kind: "rfc".to_string(),
            id: "RFC-0001".to_string(),
            title: "TUI cockpit".to_string(),
            path: "gov/rfc/RFC-0001/rfc.toml".to_string(),
            snippet: "TUI cockpit".to_string(),
            score: None,
            status: Some("normative".to_string()),
        });

        app.enter_search_result_at(0);

        assert_eq!(app.view, View::RfcDetail(0));
    }

    #[test]
    fn diagnostic_target_enters_matching_work_detail() {
        let mut app = App::new(project_index());
        app.view = View::DiagnosticList;
        app.supplement.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "work item diagnostic",
            "gov/work/WI-2026-06-06-001.toml",
        ));

        app.enter_diagnostic_target_at(0);

        assert_eq!(app.view, View::WorkDetail(0));
    }

    #[test]
    fn diagnostic_target_enters_matching_clause_detail() {
        let mut app = App::new(project_index());
        app.view = View::DiagnosticList;
        app.supplement.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "clause diagnostic",
            "gov/rfc/RFC-0001/clauses/C-TEST.toml",
        ));

        app.enter_diagnostic_target_at(0);

        assert_eq!(app.view, View::ClauseDetail(0, 0));
    }

    #[test]
    fn diagnostic_target_enters_matching_guard_detail() {
        let mut app = App::new(project_index());
        app.view = View::DiagnosticList;
        app.supplement.guards.push(guard_entry());
        app.supplement.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "guard diagnostic",
            "gov/guard/GUARD-TEST.toml",
        ));

        app.enter_diagnostic_target_at(0);

        assert_eq!(app.view, View::GuardDetail(0));
    }

    #[test]
    fn diagnostic_filter_matches_severity() {
        let mut app = App::new(project_index());
        app.view = View::DiagnosticList;
        app.filter_query = "warning".to_string();
        app.supplement.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "error diagnostic",
            "gov/rfc/RFC-0001/rfc.toml",
        ));
        app.supplement.diagnostics.push(Diagnostic::new(
            DiagnosticCode::W0110SchemaOutdated,
            "warning diagnostic",
            "gov/config.toml",
        ));

        assert_eq!(app.list_indices(), vec![1]);
    }

    #[test]
    fn filtered_diagnostic_enters_visible_target() {
        let mut app = App::new(project_index());
        app.view = View::DiagnosticList;
        app.filter_query = "target".to_string();
        app.supplement.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "other diagnostic",
            "gov/rfc/RFC-0001/rfc.toml",
        ));
        app.supplement.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "target diagnostic",
            "gov/work/WI-2026-06-06-001.toml",
        ));

        app.enter_detail();

        assert_eq!(app.view, View::WorkDetail(0));
    }

    #[test]
    fn filtered_search_enters_visible_result() {
        let mut app = App::new(project_index());
        app.view = View::Search;
        app.filter_query = "target".to_string();
        app.search_results.push(SearchResult {
            kind: "work".to_string(),
            id: "WI-2026-06-06-001".to_string(),
            title: "Other result".to_string(),
            path: "gov/work/WI-2026-06-06-001.toml".to_string(),
            snippet: "not the match".to_string(),
            score: None,
            status: Some("active".to_string()),
        });
        app.search_results.push(SearchResult {
            kind: "rfc".to_string(),
            id: "RFC-0001".to_string(),
            title: "Target result".to_string(),
            path: "gov/rfc/RFC-0001/rfc.toml".to_string(),
            snippet: "target".to_string(),
            score: None,
            status: Some("normative".to_string()),
        });

        app.enter_detail();

        assert_eq!(app.view, View::RfcDetail(0));
    }

    #[test]
    fn loop_detail_uses_independent_work_selection() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(ProjectIndex::default());
        app.supplement
            .loops
            .push(loop_entry("LOOP-2026-06-06-001", "WI-2026-06-06-001")?);
        app.supplement
            .loops
            .push(loop_entry("LOOP-2026-06-06-002", "WI-2026-06-06-002")?);
        app.view = View::LoopList;
        app.selected = 1;

        app.enter_detail();

        assert_eq!(app.view, View::LoopDetail(1));
        assert_eq!(app.selected, 1);
        assert_eq!(app.loop_selected, 0);
        assert_eq!(
            app.selected_loop_work_id(1).as_deref(),
            Some("WI-2026-06-06-002")
        );

        app.go_back();

        assert_eq!(app.view, View::LoopList);
        assert_eq!(app.selected, 1);
        Ok(())
    }

    fn project_index() -> ProjectIndex {
        ProjectIndex {
            rfcs: vec![RfcIndex {
                rfc: RfcSpec {
                    rfc_id: "RFC-0001".to_string(),
                    title: "TUI cockpit".to_string(),
                    version: "0.1.0".to_string(),
                    status: RfcStatus::Normative,
                    phase: RfcPhase::Impl,
                    owners: vec![],
                    created: "2026-06-06".to_string(),
                    updated: None,
                    supersedes: None,
                    refs: vec![],
                    tags: vec![],
                    sections: vec![],
                    changelog: vec![],
                    signature: None,
                },
                clauses: vec![ClauseEntry {
                    spec: ClauseSpec {
                        clause_id: "C-TEST".to_string(),
                        title: "Clause test".to_string(),
                        kind: ClauseKind::Normative,
                        status: ClauseStatus::Active,
                        text: "Clause body".to_string(),
                        anchors: vec![],
                        superseded_by: None,
                        since: None,
                        tags: vec![],
                    },
                    path: PathBuf::from("gov/rfc/RFC-0001/clauses/C-TEST.toml"),
                }],
                path: PathBuf::from("gov/rfc/RFC-0001/rfc.toml"),
            }],
            adrs: vec![],
            work_items: vec![WorkItemEntry {
                spec: WorkItemSpec {
                    govctl: WorkItemMeta::new(
                        "WI-2026-06-06-001",
                        "Implement TUI v2",
                        WorkItemStatus::Active,
                    ),
                    content: WorkItemContent::default(),
                    verification: WorkItemVerification::default(),
                },
                path: PathBuf::from("gov/work/WI-2026-06-06-001.toml"),
            }],
        }
    }

    fn rfc_entry(rfc_id: &str, clause_id: &str) -> RfcIndex {
        RfcIndex {
            rfc: RfcSpec {
                rfc_id: rfc_id.to_string(),
                title: rfc_id.to_string(),
                version: "0.1.0".to_string(),
                status: RfcStatus::Normative,
                phase: RfcPhase::Impl,
                owners: vec![],
                created: "2026-06-06".to_string(),
                updated: None,
                supersedes: None,
                refs: vec![],
                tags: vec![],
                sections: vec![],
                changelog: vec![],
                signature: None,
            },
            clauses: vec![ClauseEntry {
                spec: ClauseSpec {
                    clause_id: clause_id.to_string(),
                    title: clause_id.to_string(),
                    kind: ClauseKind::Normative,
                    status: ClauseStatus::Active,
                    text: "Clause body".to_string(),
                    anchors: vec![],
                    superseded_by: None,
                    since: None,
                    tags: vec![],
                },
                path: PathBuf::from(format!("gov/rfc/{rfc_id}/clauses/{clause_id}.toml")),
            }],
            path: PathBuf::from(format!("gov/rfc/{rfc_id}/rfc.toml")),
        }
    }

    fn guard_entry() -> GuardEntry {
        GuardEntry {
            spec: GuardSpec {
                govctl: GuardMeta::new("GUARD-TEST", "Guard test"),
                check: GuardCheck {
                    command: "true".to_string(),
                    timeout_secs: 1,
                    pattern: None,
                },
            },
            path: PathBuf::from("gov/guard/GUARD-TEST.toml"),
        }
    }

    fn loop_entry(
        loop_id: &str,
        work_id: &str,
    ) -> crate::diagnostic::DiagnosticResult<TuiLoopEntry> {
        let mut dependencies = BTreeMap::new();
        dependencies.insert(work_id.to_string(), Vec::new());
        Ok(TuiLoopEntry {
            id: loop_id.to_string(),
            state: Some(LoopState::new(
                loop_id,
                vec![work_id.to_string()],
                vec![work_id.to_string()],
                dependencies,
            )?),
            diagnostic: None,
        })
    }
}
