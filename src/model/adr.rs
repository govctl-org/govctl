use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

/// ADR metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrMeta {
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub status: AdrStatus,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Status for ADR alternatives
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AlternativeStatus {
    #[default]
    Considered,
    Rejected,
    Accepted,
}

/// An alternative option considered in an ADR.
/// Extended per [[ADR-0027]] with pros, cons, and rejection_reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub text: String,
    #[serde(default)]
    pub status: AlternativeStatus,
    /// Advantages of this alternative per [[ADR-0027]]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pros: Vec<String>,
    /// Disadvantages of this alternative per [[ADR-0027]]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cons: Vec<String>,
    /// If rejected, explains why per [[ADR-0027]]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

impl Alternative {
    #[cfg(test)]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: AlternativeStatus::Considered,
            pros: vec![],
            cons: vec![],
            rejection_reason: None,
        }
    }
}

/// ADR content section `[content]`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdrContent {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub decision: String,
    #[serde(default)]
    pub consequences: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<Alternative>,
}

/// Complete ADR file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrSpec {
    pub govctl: AdrMeta,
    pub content: AdrContent,
}

/// ADR status lifecycle: proposed -> accepted -> superseded
///                                -> rejected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}
