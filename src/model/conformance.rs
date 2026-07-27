use serde::{Deserialize, Serialize};

/// Conformance Case metadata section `[govctl]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceMeta {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Versioned RFC Clause binding owned by a Conformance Case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementBinding {
    #[serde(rename = "ref")]
    pub clause_ref: String,
    pub version: String,
}

/// Project-owned scenario locator and its declared trace edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceContent {
    pub path: String,
    pub selector: String,
    pub requirements: Vec<RequirementBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guards: Vec<String>,
}

/// Complete Conformance Case file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceSpec {
    pub govctl: ConformanceMeta,
    pub case: ConformanceContent,
}
