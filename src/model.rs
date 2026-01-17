//! Data models for all governed artifacts.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

// =============================================================================
// RFC Models (JSON SSOT)
// =============================================================================

/// RFC specification (rfc.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfcSpec {
    pub rfc_id: String,
    pub title: String,
    pub version: String,
    pub status: RfcStatus,
    pub phase: RfcPhase,
    pub owners: Vec<String>,
    pub created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub sections: Vec<SectionSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changelog: Vec<ChangelogEntry>,
}

/// Section within an RFC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionSpec {
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clauses: Vec<String>,
}

/// Individual clause specification (C-*.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseSpec {
    pub clause_id: String,
    pub title: String,
    pub kind: ClauseKind,
    #[serde(default)]
    pub status: ClauseStatus,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

/// Changelog entry for RFC versioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<String>,
}

/// RFC status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum RfcStatus {
    Draft,
    Normative,
    Deprecated,
}

/// RFC phase lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum RfcPhase {
    Spec,
    Impl,
    Test,
    Stable,
}

/// Clause kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsRefStr, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ClauseKind {
    Normative,
    Informative,
}

/// Clause status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ClauseStatus {
    #[default]
    Active,
    Deprecated,
    Superseded,
}

// =============================================================================
// ADR Models (TOML SSOT)
// =============================================================================

/// ADR metadata section [govctl]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrMeta {
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub status: AdrStatus,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// ADR content section [content]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdrContent {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub decision: String,
    #[serde(default)]
    pub consequences: String,
}

/// Complete ADR file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrSpec {
    pub govctl: AdrMeta,
    pub content: AdrContent,
}

/// ADR status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Superseded,
    Deprecated,
}

// =============================================================================
// Work Item Models (TOML SSOT)
// =============================================================================

/// Work Item metadata section [govctl]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemMeta {
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub status: WorkItemStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// Work Item content section [content]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkItemContent {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub notes: String,
}

/// Complete Work Item file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemSpec {
    pub govctl: WorkItemMeta,
    pub content: WorkItemContent,
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
