use serde::{Deserialize, Serialize};

/// Release file metadata section `[govctl]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasesMeta {
    #[serde(
        default = "default_schema_version",
        rename = "schema",
        skip_serializing
    )]
    _schema: u32,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for ReleasesMeta {
    fn default() -> Self {
        Self {
            _schema: default_schema_version(),
        }
    }
}

/// A single release entry
/// Per [[ADR-0014]], tracks which work items belong to which version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub date: String,
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
