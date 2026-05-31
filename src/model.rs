//! Data models for all governed artifacts.
//!
//! Implements data structures per [[RFC-0000]] governance framework:
//! - RFCs with clauses ([[RFC-0000:C-RFC-DEF]])
//! - ADRs ([[RFC-0000:C-ADR-DEF]])
//! - Work Items ([[RFC-0000:C-WORK-DEF]])
//!
//! Lifecycle state machines per [[RFC-0001]].

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

mod adr;
mod rfc;

pub use adr::{AdrContent, AdrMeta, AdrSpec, AdrStatus, Alternative, AlternativeStatus};
pub use rfc::{
    ChangelogEntry, ClauseKind, ClauseSpec, ClauseStatus, ClauseWire, RfcPhase, RfcSpec, RfcStatus,
    RfcWire, SectionSpec,
};

// =============================================================================
// Work Item Models (TOML SSOT)
// =============================================================================

/// Work Item metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemMeta {
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub status: WorkItemStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Work item-specific verification policy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkItemVerification {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_guards: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waivers: Vec<GuardWaiver>,
}

impl WorkItemVerification {
    pub fn is_empty(&self) -> bool {
        self.required_guards.is_empty() && self.waivers.is_empty()
    }
}

/// Explicit waiver for one required verification guard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardWaiver {
    pub guard: String,
    pub reason: String,
}

/// Status for checklist items (acceptance criteria, decisions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ChecklistStatus {
    #[default]
    Pending,
    Done,
    Cancelled,
}

/// A checklist item with text, status, and changelog category
/// Per [[ADR-0013]], category enables changelog generation from acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub text: String,
    #[serde(default)]
    pub status: ChecklistStatus,
    #[serde(default)]
    pub category: ChangelogCategory,
}

impl ChecklistItem {
    /// Create a checklist item with default category (Added)
    #[allow(dead_code)] // Used in tests; kept as public API for simpler construction
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: ChecklistStatus::Pending,
            category: ChangelogCategory::default(),
        }
    }

    /// Create a checklist item with a specific category
    pub fn with_category(text: impl Into<String>, category: ChangelogCategory) -> Self {
        Self {
            text: text.into(),
            status: ChecklistStatus::Pending,
            category,
        }
    }
}

/// A legacy inline journal entry preserved for work item rendering per [[ADR-0047]].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// ISO date string "YYYY-MM-DD"
    pub date: String,
    /// Optional topic/module identifier for this entry
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Markdown text with progress details
    pub content: String,
}

/// Work Item content section `[content]`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkItemContent {
    #[serde(default)]
    pub description: String,
    /// Legacy inline journal entries are parsed for render/show only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub journal: Vec<JournalEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance_criteria: Vec<ChecklistItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Complete Work Item file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemSpec {
    pub govctl: WorkItemMeta,
    pub content: WorkItemContent,
    #[serde(default, skip_serializing_if = "WorkItemVerification::is_empty")]
    pub verification: WorkItemVerification,
}

/// Work Item status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WorkItemStatus {
    Queue,
    Active,
    Done,
    Cancelled,
}

/// Changelog category for Keep a Changelog format.
/// Used for both RFC changelog entries and work item acceptance criteria.
/// Per [[ADR-0012]] and [[ADR-0013]].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, AsRefStr, ValueEnum,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ChangelogCategory {
    #[default]
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
    /// Internal/housekeeping items - excluded from rendered changelog
    Chore,
}

// =============================================================================
// Verification Guard Models (TOML SSOT)
// =============================================================================

/// Verification Guard metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardMeta {
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub schema: u32,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Executable check for a verification guard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardCheck {
    pub command: String,
    #[serde(default = "default_guard_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

fn default_guard_timeout_secs() -> u64 {
    300
}

/// Complete Verification Guard file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardSpec {
    pub govctl: GuardMeta,
    pub check: GuardCheck,
}

impl ChangelogCategory {
    /// Canonical category prefixes shown in error messages and docs.
    /// All verb forms for consistency with imperative acceptance criteria.
    pub const VALID_PREFIXES: &'static [&'static str] = &[
        "add",
        "fix",
        "change",
        "remove",
        "deprecate",
        "security",
        "chore",
    ];

    /// Parse a prefix string into a category.
    ///
    /// Accepts canonical Keep-a-Changelog names, verb forms, and
    /// conventional-commit aliases so agents and humans can use
    /// whichever style they reach for first.
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix.to_lowercase().as_str() {
            // Added: new features and capabilities
            "add" | "added" | "feat" | "feature" => Some(Self::Added),
            // Changed: modifications to existing behavior
            "changed" | "change" | "refactor" | "perf" => Some(Self::Changed),
            // Deprecated: features marked for future removal
            "deprecated" | "deprecate" => Some(Self::Deprecated),
            // Removed: deleted features
            "removed" | "remove" => Some(Self::Removed),
            // Fixed: bug fixes
            "fix" | "fixed" => Some(Self::Fixed),
            // Security: vulnerability fixes
            "security" | "sec" => Some(Self::Security),
            // Chore: internal tasks excluded from changelog
            "chore" | "internal" | "test" | "tests" | "doc" | "docs" | "ci" | "build" => {
                Some(Self::Chore)
            }
            _ => None,
        }
    }
}

// =============================================================================
// Release Models (TOML - gov/releases.toml)
// =============================================================================

/// Release file metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasesMeta {
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub schema: u32,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for ReleasesMeta {
    fn default() -> Self {
        Self {
            schema: default_schema_version(),
        }
    }
}

/// A single release entry
/// Per [[ADR-0014]], tracks which work items belong to which version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub date: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// Collection of releases in gov/releases.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleasesFile {
    #[serde(default)]
    pub govctl: ReleasesMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<Release>,
}

// =============================================================================
// Indexed structures for loaded data
// =============================================================================

/// Loaded RFC with all its clauses
#[derive(Debug, Clone)]
pub struct RfcIndex {
    pub rfc: RfcSpec,
    pub clauses: Vec<ClauseEntry>,
    pub path: std::path::PathBuf,
}

/// Clause with its path
#[derive(Debug, Clone)]
pub struct ClauseEntry {
    pub spec: ClauseSpec,
    pub path: std::path::PathBuf,
}

/// Loaded ADR with full spec
#[derive(Debug, Clone)]
pub struct AdrEntry {
    pub spec: AdrSpec,
    pub path: std::path::PathBuf,
}

impl AdrEntry {
    /// Convenience accessor for metadata
    pub fn meta(&self) -> &AdrMeta {
        &self.spec.govctl
    }
}

/// Loaded Work Item with full spec
#[derive(Debug, Clone)]
pub struct WorkItemEntry {
    pub spec: WorkItemSpec,
    pub path: std::path::PathBuf,
}

impl WorkItemEntry {
    /// Convenience accessor for metadata
    pub fn meta(&self) -> &WorkItemMeta {
        &self.spec.govctl
    }
}

/// Loaded Verification Guard with full spec.
#[derive(Debug, Clone)]
pub struct GuardEntry {
    pub spec: GuardSpec,
    pub path: std::path::PathBuf,
}

impl GuardEntry {
    pub fn meta(&self) -> &GuardMeta {
        &self.spec.govctl
    }
}

/// Full project index
#[derive(Debug, Clone, Default)]
pub struct ProjectIndex {
    pub rfcs: Vec<RfcIndex>,
    pub adrs: Vec<AdrEntry>,
    pub work_items: Vec<WorkItemEntry>,
}

impl ProjectIndex {
    /// Iterate over all clauses across all RFCs
    pub fn iter_clauses(&self) -> impl Iterator<Item = (&RfcIndex, &ClauseEntry)> {
        self.rfcs
            .iter()
            .flat_map(|rfc| rfc.clauses.iter().map(move |c| (rfc, c)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ChecklistItem Tests
    // =========================================================================

    #[test]
    fn test_checklist_item_new() {
        let item = ChecklistItem::new("Test criterion");
        assert_eq!(item.text, "Test criterion");
        assert_eq!(item.status, ChecklistStatus::Pending);
    }

    #[test]
    fn test_checklist_item_new_from_string() {
        let item = ChecklistItem::new(String::from("From String"));
        assert_eq!(item.text, "From String");
        assert_eq!(item.status, ChecklistStatus::Pending);
    }

    // =========================================================================
    // Alternative Tests
    // =========================================================================

    #[test]
    fn test_alternative_new() {
        let alt = Alternative::new("Use Redis for caching");
        assert_eq!(alt.text, "Use Redis for caching");
        assert_eq!(alt.status, AlternativeStatus::Considered);
    }

    #[test]
    fn test_alternative_new_from_string() {
        let alt = Alternative::new(String::from("Use PostgreSQL"));
        assert_eq!(alt.text, "Use PostgreSQL");
        assert_eq!(alt.status, AlternativeStatus::Considered);
    }

    // =========================================================================
    // Enum Default Tests
    // =========================================================================

    #[test]
    fn test_checklist_status_default() {
        assert_eq!(ChecklistStatus::default(), ChecklistStatus::Pending);
    }

    #[test]
    fn test_alternative_status_default() {
        assert_eq!(AlternativeStatus::default(), AlternativeStatus::Considered);
    }

    #[test]
    fn test_clause_status_default() {
        assert_eq!(ClauseStatus::default(), ClauseStatus::Active);
    }

    // =========================================================================
    // AsRef Tests (strum)
    // =========================================================================

    #[test]
    fn test_rfc_status_as_ref() {
        assert_eq!(RfcStatus::Draft.as_ref(), "draft");
        assert_eq!(RfcStatus::Normative.as_ref(), "normative");
        assert_eq!(RfcStatus::Deprecated.as_ref(), "deprecated");
    }

    #[test]
    fn test_rfc_phase_as_ref() {
        assert_eq!(RfcPhase::Spec.as_ref(), "spec");
        assert_eq!(RfcPhase::Impl.as_ref(), "impl");
        assert_eq!(RfcPhase::Test.as_ref(), "test");
        assert_eq!(RfcPhase::Stable.as_ref(), "stable");
    }

    #[test]
    fn test_work_item_status_as_ref() {
        assert_eq!(WorkItemStatus::Queue.as_ref(), "queue");
        assert_eq!(WorkItemStatus::Active.as_ref(), "active");
        assert_eq!(WorkItemStatus::Done.as_ref(), "done");
        assert_eq!(WorkItemStatus::Cancelled.as_ref(), "cancelled");
    }

    #[test]
    fn test_adr_status_as_ref() {
        assert_eq!(AdrStatus::Proposed.as_ref(), "proposed");
        assert_eq!(AdrStatus::Accepted.as_ref(), "accepted");
        assert_eq!(AdrStatus::Superseded.as_ref(), "superseded");
    }

    #[test]
    fn test_checklist_status_as_ref() {
        assert_eq!(ChecklistStatus::Pending.as_ref(), "pending");
        assert_eq!(ChecklistStatus::Done.as_ref(), "done");
        assert_eq!(ChecklistStatus::Cancelled.as_ref(), "cancelled");
    }

    #[test]
    fn test_alternative_status_as_ref() {
        assert_eq!(AlternativeStatus::Considered.as_ref(), "considered");
        assert_eq!(AlternativeStatus::Rejected.as_ref(), "rejected");
        assert_eq!(AlternativeStatus::Accepted.as_ref(), "accepted");
    }

    // =========================================================================
    // AdrEntry/WorkItemEntry accessor Tests
    // =========================================================================

    #[test]
    fn test_adr_entry_meta_accessor() {
        let entry = AdrEntry {
            spec: AdrSpec {
                govctl: AdrMeta {
                    schema: 1,
                    id: "ADR-0001".to_string(),
                    title: "Test ADR".to_string(),
                    status: AdrStatus::Proposed,
                    date: "2026-01-17".to_string(),
                    superseded_by: None,
                    refs: vec![],
                    tags: vec![],
                },
                content: AdrContent::default(),
            },
            path: std::path::PathBuf::from("test.toml"),
        };
        assert_eq!(entry.meta().id, "ADR-0001");
        assert_eq!(entry.meta().title, "Test ADR");
    }

    #[test]
    fn test_work_item_entry_meta_accessor() {
        let entry = WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 1,
                    id: "WI-2026-01-17-001".to_string(),
                    title: "Test Work Item".to_string(),
                    status: WorkItemStatus::Queue,
                    created: Some("2026-01-17".to_string()),
                    started: None,
                    completed: None,
                    refs: vec![],
                    depends_on: vec![],
                    tags: vec![],
                },
                content: WorkItemContent::default(),
                verification: WorkItemVerification::default(),
            },
            path: std::path::PathBuf::from("test.toml"),
        };
        assert_eq!(entry.meta().id, "WI-2026-01-17-001");
        assert_eq!(entry.meta().status, WorkItemStatus::Queue);
    }
}
