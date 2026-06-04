use serde::{Deserialize, Serialize};

/// Verification Guard metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardMeta {
    #[serde(default, rename = "schema", skip_serializing)]
    _schema: u32,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl GuardMeta {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            _schema: 1,
            id: id.into(),
            title: title.into(),
            refs: vec![],
            tags: vec![],
        }
    }
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
