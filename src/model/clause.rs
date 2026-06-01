use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

/// Individual clause specification.
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
