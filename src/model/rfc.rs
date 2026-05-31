use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

/// RFC specification — runtime domain model used throughout the codebase.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub sections: Vec<SectionSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changelog: Vec<ChangelogEntry>,
    /// Content signature for amendment detection per [[ADR-0016]]
    /// SHA-256 hash of canonical RFC content at last released version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// RFC TOML wire format: `[govctl]` metadata + top-level `[[sections]]` / `[[changelog]]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfcWire {
    pub govctl: RfcMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<SectionSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changelog: Vec<ChangelogEntry>,
}

/// RFC metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfcMeta {
    /// Backward-compat: accepted on read, never written. See `config.toml [schema] version`.
    #[serde(default, rename = "schema", skip_serializing)]
    _schema: u32,
    pub id: String,
    pub title: String,
    pub version: String,
    pub status: RfcStatus,
    pub phase: RfcPhase,
    pub owners: Vec<String>,
    pub created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl From<RfcSpec> for RfcWire {
    fn from(s: RfcSpec) -> Self {
        Self {
            govctl: RfcMeta {
                _schema: 1,
                id: s.rfc_id,
                title: s.title,
                version: s.version,
                status: s.status,
                phase: s.phase,
                owners: s.owners,
                created: s.created,
                updated: s.updated,
                supersedes: s.supersedes,
                refs: s.refs,
                tags: s.tags,
                signature: s.signature,
            },
            sections: s.sections,
            changelog: s.changelog,
        }
    }
}

impl From<RfcWire> for RfcSpec {
    fn from(w: RfcWire) -> Self {
        Self {
            rfc_id: w.govctl.id,
            title: w.govctl.title,
            version: w.govctl.version,
            status: w.govctl.status,
            phase: w.govctl.phase,
            owners: w.govctl.owners,
            created: w.govctl.created,
            updated: w.govctl.updated,
            supersedes: w.govctl.supersedes,
            refs: w.govctl.refs,
            tags: w.govctl.tags,
            sections: w.sections,
            changelog: w.changelog,
            signature: w.govctl.signature,
        }
    }
}

/// Clause TOML wire format: `[govctl]` metadata + `[content]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseWire {
    pub govctl: ClauseMeta,
    pub content: ClauseContent,
}

/// Clause metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseMeta {
    #[serde(default, rename = "schema", skip_serializing)]
    _schema: u32,
    pub id: String,
    pub title: String,
    pub kind: ClauseKind,
    #[serde(default)]
    pub status: ClauseStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Clause content section `[content]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseContent {
    pub text: String,
}

impl From<ClauseSpec> for ClauseWire {
    fn from(s: ClauseSpec) -> Self {
        Self {
            govctl: ClauseMeta {
                _schema: 1,
                id: s.clause_id,
                title: s.title,
                kind: s.kind,
                status: s.status,
                anchors: s.anchors,
                superseded_by: s.superseded_by,
                since: s.since,
                tags: s.tags,
            },
            content: ClauseContent { text: s.text },
        }
    }
}

impl From<ClauseWire> for ClauseSpec {
    fn from(w: ClauseWire) -> Self {
        Self {
            clause_id: w.govctl.id,
            title: w.govctl.title,
            kind: w.govctl.kind,
            status: w.govctl.status,
            text: w.content.text,
            anchors: w.govctl.anchors,
            superseded_by: w.govctl.superseded_by,
            since: w.govctl.since,
            tags: w.govctl.tags,
        }
    }
}

/// Changelog entry for RFC versioning (Keep a Changelog format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    /// Optional freeform notes for this release
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// New features added
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<String>,
    /// Changes to existing functionality
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<String>,
    /// Features marked for removal in upcoming releases
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deprecated: Vec<String>,
    /// Features removed in this release
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// Bug fixes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed: Vec<String>,
    /// Security-related changes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<String>,
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
