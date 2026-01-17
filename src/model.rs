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
// ADR Models (Markdown SSOT)
// =============================================================================

/// ADR frontmatter (under phaseos: namespace)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrMeta {
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub kind: String, // Should be "adr"
    pub status: AdrStatus,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
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
// Work Item Models (Markdown SSOT)
// =============================================================================

/// Work Item frontmatter (under phaseos: namespace)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemMeta {
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub kind: String, // Should be "work"
    pub status: WorkItemStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// Work Item status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WorkItemStatus {
    Queue,
    Active,
    Done,
}

// =============================================================================
// Wrapper for frontmatter namespace
// =============================================================================

/// Wrapper for phaseos: namespace in frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseOsWrapper<T> {
    pub phaseos: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_yaml::Value>,
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

/// Loaded ADR with metadata
#[derive(Debug, Clone)]
pub struct AdrEntry {
    pub meta: AdrMeta,
    pub path: std::path::PathBuf,
    #[allow(dead_code)]
    pub content: String,
}

/// Loaded Work Item with metadata
#[derive(Debug, Clone)]
pub struct WorkItemEntry {
    pub meta: WorkItemMeta,
    pub path: std::path::PathBuf,
    #[allow(dead_code)]
    pub content: String,
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
