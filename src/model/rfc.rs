use super::changelog::ChangelogEntry;
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
    /// Sealed content signature for amendment detection per [[ADR-0016]].
    /// Set when the current version advances from spec to impl.
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
