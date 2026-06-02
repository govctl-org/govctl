//! Configuration loading and management.
//!
//! Implements [[ADR-0009]] configurable source code reference scanning.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod id_strategy;
mod runtime;
mod template;

pub use id_strategy::IdStrategy;

/// Project configuration (gov/config.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Absolute path to `gov/` — not configurable, derived from config file location.
    #[serde(skip)]
    pub gov_root: PathBuf,
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub schema: SchemaConfig,
    #[serde(default)]
    pub source_scan: SourceScanConfig,
    #[serde(default)]
    pub work_item: WorkItemConfig,
    #[serde(default)]
    pub verification: VerificationConfig,
    #[serde(default)]
    pub concurrency: ConcurrencyConfig,
    #[serde(default)]
    pub tags: TagsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gov_root: PathBuf::from("gov"),
            project: ProjectConfig::default(),
            paths: PathsConfig::default(),
            schema: SchemaConfig::default(),
            source_scan: SourceScanConfig::default(),
            work_item: WorkItemConfig::default(),
            verification: VerificationConfig::default(),
            concurrency: ConcurrencyConfig::default(),
            tags: TagsConfig::default(),
        }
    }
}

/// Controlled-vocabulary tag configuration.
///
/// Defines the allowed tag set for the project. Artifacts may only use tags
/// listed here. Implements [[RFC-0002:C-RESOURCES]] controlled-vocabulary tags.
/// An empty `allowed` list means no tags are permitted (deny-all).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagsConfig {
    /// Allowed tag values (each must match `^[a-z][a-z0-9-]*$`).
    /// Empty = deny all.
    #[serde(default)]
    pub allowed: Vec<String>,
}

/// Project-level verification guard policy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub default_guards: Vec<String>,
}

/// Concurrency and write-safety configuration.
///
/// Implements [[RFC-0004]] concurrent write safety (configurable lock timeout).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    /// Maximum seconds to wait for exclusive access before failing (default: 30).
    #[serde(default = "default_lock_timeout_secs")]
    pub lock_timeout_secs: u64,
}

fn default_lock_timeout_secs() -> u64 {
    30
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            lock_timeout_secs: default_lock_timeout_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default = "default_project_name")]
    pub name: String,
    /// Default owner for new RFCs (e.g., "@your-handle" or "@org-name")
    #[serde(default = "default_owner")]
    pub default_owner: String,
}

fn default_project_name() -> String {
    "govctl-project".to_string()
}

fn default_owner() -> String {
    // Check environment variable first (useful for testing)
    if let Ok(owner) = std::env::var("GOVCTL_DEFAULT_OWNER") {
        return owner;
    }

    // Try to get git user.name, fall back to placeholder.
    // The value becomes an owner handle, so "@" alone is not useful.
    git_config_value("user.name")
        .map(|name| format!("@{name}"))
        .filter(|owner| owner.len() > 1)
        .unwrap_or_else(|| "@your-handle".to_string())
}

fn git_config_value(key: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", key])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    Some(value.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Output directory for rendered docs (docs/)
    #[serde(default = "default_docs_output")]
    pub docs_output: PathBuf,
    /// AI agent directory (Claude, Cursor, Windsurf, etc.)
    /// Contains skills/ and agents/ subdirs, written by `init-skills`
    #[serde(default = "default_agent_dir")]
    pub agent_dir: PathBuf,
}

fn default_docs_output() -> PathBuf {
    PathBuf::from("docs")
}

pub fn default_agent_dir() -> PathBuf {
    PathBuf::from(".claude")
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            docs_output: default_docs_output(),
            agent_dir: default_agent_dir(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaConfig {
    #[serde(default = "default_schema_version")]
    pub version: u32,
}

fn default_schema_version() -> u32 {
    crate::cmd::migrate::CURRENT_SCHEMA_VERSION
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            version: default_schema_version(),
        }
    }
}

/// Source code scanning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceScanConfig {
    /// Enable source code scanning (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Glob patterns for files to include (e.g., "src/**/*.rs")
    #[serde(default = "default_scan_include")]
    pub include: Vec<String>,
    /// Glob patterns for files to exclude (e.g., "**/tests/**")
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Regex pattern with capture group 1 for artifact ID
    #[serde(default = "default_scan_pattern")]
    pub pattern: String,
}

fn default_scan_include() -> Vec<String> {
    vec![
        "src/**/*.rs".to_string(),
        "crates/**/*.rs".to_string(),
        "**/*.md".to_string(),
    ]
}

fn default_scan_pattern() -> String {
    // Matches double-bracket references:
    // - [[RFC-NNNN]] or [[RFC-NNNN:C-CLAUSE]]
    // - [[ADR-NNNN]]
    // - [[WI-YYYY-MM-DD-NNN]] (sequential)
    // - [[WI-YYYY-MM-DD-HHHH-NNN]] (author-hash)
    // - [[WI-YYYY-MM-DD-HHHH]] (random)
    r"\[\[(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3}))\]\]".to_string()
}

impl Default for SourceScanConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            include: default_scan_include(),
            exclude: vec![],
            pattern: default_scan_pattern(),
        }
    }
}

/// Work item configuration
///
/// Implements [[ADR-0020]] configurable work item ID strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemConfig {
    /// ID generation strategy (default: sequential)
    #[serde(default)]
    pub id_strategy: IdStrategy,
}

impl Default for WorkItemConfig {
    fn default() -> Self {
        Self {
            id_strategy: IdStrategy::Sequential,
        }
    }
}
