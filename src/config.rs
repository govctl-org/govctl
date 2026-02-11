//! Configuration loading and management.
//!
//! Implements [[ADR-0009]] configurable source code reference scanning.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Project configuration (gov/config.toml)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
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
    // Try to get git user.name, fall back to placeholder
    std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| format!("@{}", s.trim()))
        .filter(|s| s.len() > 1) // "@" alone is not valid
        .unwrap_or_else(|| "@your-handle".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Root directory for governance SSOT (gov/)
    #[serde(default = "default_gov_root")]
    pub gov_root: PathBuf,
    /// Output directory for rendered docs (docs/)
    #[serde(default = "default_docs_output")]
    pub docs_output: PathBuf,
    /// Commands directory for AI IDEs (Claude, Cursor, Windsurf, etc.)
    #[serde(default = "default_commands_dir")]
    pub commands_dir: PathBuf,
}

fn default_gov_root() -> PathBuf {
    PathBuf::from("gov")
}

fn default_docs_output() -> PathBuf {
    PathBuf::from("docs")
}

fn default_commands_dir() -> PathBuf {
    PathBuf::from(".claude/commands")
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            gov_root: default_gov_root(),
            docs_output: default_docs_output(),
            commands_dir: default_commands_dir(),
        }
    }
}

impl PathsConfig {
    /// RFC SSOT directory (gov/rfc/)
    pub fn rfc_dir(&self) -> PathBuf {
        self.gov_root.join("rfc")
    }

    /// ADR SSOT directory (gov/adr/)
    pub fn adr_dir(&self) -> PathBuf {
        self.gov_root.join("adr")
    }

    /// Work item SSOT directory (gov/work/)
    pub fn work_dir(&self) -> PathBuf {
        self.gov_root.join("work")
    }

    /// Schema directory (gov/schema/)
    pub fn schema_dir(&self) -> PathBuf {
        self.gov_root.join("schema")
    }

    /// Templates directory (gov/templates/)
    pub fn templates_dir(&self) -> PathBuf {
        self.gov_root.join("templates")
    }

    /// RFC rendered output (docs/rfc/)
    pub fn rfc_output(&self) -> PathBuf {
        self.docs_output.join("rfc")
    }

    /// ADR rendered output (docs/adr/)
    pub fn adr_output(&self) -> PathBuf {
        self.docs_output.join("adr")
    }

    /// Work item rendered output (docs/work/)
    pub fn work_output(&self) -> PathBuf {
        self.docs_output.join("work")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaConfig {
    #[serde(default = "default_schema_version")]
    pub version: u32,
}

fn default_schema_version() -> u32 {
    1
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

/// Work item ID generation strategy
///
/// - `Sequential`: `WI-YYYY-MM-DD-NNN` (default, for solo projects)
/// - `AuthorHash`: `WI-YYYY-MM-DD-{hash4}-NNN` (for multi-person teams)
/// - `Random`: `WI-YYYY-MM-DD-{rand4}` (simple uniqueness)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdStrategy {
    /// Sequential numbering per day (default)
    #[default]
    Sequential,
    /// Hash of git user.email + sequential numbering per author
    AuthorHash,
    /// Random 4-char hex suffix (no sequence number)
    Random,
}

impl IdStrategy {
    /// Get the author hash (first 4 chars of sha256(git user.email))
    pub fn get_author_hash() -> Option<String> {
        let output = std::process::Command::new("git")
            .args(["config", "user.email"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let email = String::from_utf8(output.stdout).ok()?;
        let email = email.trim();
        if email.is_empty() {
            return None;
        }

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(email.as_bytes());
        let result = hasher.finalize();
        Some(format!("{:02x}{:02x}", result[0], result[1]))
    }

    /// Generate a random 4-char hex suffix
    pub fn generate_random_suffix() -> String {
        use rand::RngExt;
        let mut rng = rand::rng();
        let bytes: [u8; 2] = rng.random();
        format!("{:02x}{:02x}", bytes[0], bytes[1])
    }
}

impl Config {
    /// Load config from file or use defaults
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path
            .map(PathBuf::from)
            .or_else(Self::find_config)
            .unwrap_or_else(|| PathBuf::from("gov/config.toml"));

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config: {}", config_path.display()))?;
            Ok(config)
        } else {
            // Return default config if no file exists
            Ok(Config::default())
        }
    }

    /// Find config file by walking up directory tree
    fn find_config() -> Option<PathBuf> {
        let mut current = std::env::current_dir().ok()?;
        loop {
            let config_path = current.join("gov/config.toml");
            if config_path.exists() {
                return Some(config_path);
            }
            if !current.pop() {
                return None;
            }
        }
    }

    // Convenience accessors that delegate to paths
    pub fn rfc_dir(&self) -> PathBuf {
        self.paths.rfc_dir()
    }

    pub fn adr_dir(&self) -> PathBuf {
        self.paths.adr_dir()
    }

    pub fn work_dir(&self) -> PathBuf {
        self.paths.work_dir()
    }

    pub fn schema_dir(&self) -> PathBuf {
        self.paths.schema_dir()
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.paths.templates_dir()
    }

    pub fn rfc_output(&self) -> PathBuf {
        self.paths.rfc_output()
    }

    pub fn adr_output(&self) -> PathBuf {
        self.paths.adr_output()
    }

    pub fn work_output(&self) -> PathBuf {
        self.paths.work_output()
    }

    /// Releases file path (gov/releases.toml)
    pub fn releases_path(&self) -> PathBuf {
        self.paths.gov_root.join("releases.toml")
    }

    /// Generate default config TOML
    pub fn default_toml() -> &'static str {
        r#"[project]
name = "my-project"
# Default owner for new RFCs (uses git user.name if not set)
# default_owner = "@your-handle"

[paths]
gov_root = "gov"
docs_output = "docs"
# Commands directory for AI IDEs (Claude Desktop, Cursor, Windsurf, etc.)
# Default: ".claude/commands"
# For Cursor: ".cursor/commands"
# For Windsurf: ".windsurf/commands"
# commands_dir = ".claude/commands"

[schema]
version = 1

# [work_item]
# ID strategy for work items (default: sequential)
# - sequential: WI-YYYY-MM-DD-NNN (solo projects)
# - author-hash: WI-YYYY-MM-DD-{hash}-NNN (multi-person teams, uses git email)
# - random: WI-YYYY-MM-DD-{rand} (simple uniqueness)
# id_strategy = "author-hash"
"#
    }
}
