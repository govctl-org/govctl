use super::changelog::ChangelogCategory;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

/// Work Item metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemMeta {
    #[serde(default, rename = "schema", skip_serializing)]
    _schema: u32,
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

impl WorkItemMeta {
    pub fn new(id: impl Into<String>, title: impl Into<String>, status: WorkItemStatus) -> Self {
        Self {
            _schema: 1,
            id: id.into(),
            title: title.into(),
            status,
            created: None,
            started: None,
            completed: None,
            refs: vec![],
            depends_on: vec![],
            tags: vec![],
        }
    }
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
    #[cfg(test)]
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
