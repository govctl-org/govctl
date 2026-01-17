//! Configuration loading and management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Project configuration (phaseos.toml)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub schema: SchemaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default = "default_project_name")]
    pub name: String,
}

fn default_project_name() -> String {
    "phaseos-project".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Root directory for RFC JSON source
    #[serde(default = "default_spec_root")]
    pub spec_root: PathBuf,
    /// Output directory for rendered RFC markdown
    #[serde(default = "default_rfc_output")]
    pub rfc_output: PathBuf,
    /// Directory for ADR markdown files
    #[serde(default = "default_adr_dir")]
    pub adr_dir: PathBuf,
    /// Directory for work item markdown files
    #[serde(default = "default_work_dir")]
    pub work_dir: PathBuf,
    /// Directory for templates
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
}

fn default_spec_root() -> PathBuf {
    PathBuf::from("spec")
}

fn default_rfc_output() -> PathBuf {
    PathBuf::from("docs/rfc")
}

fn default_adr_dir() -> PathBuf {
    PathBuf::from("docs/adr")
}

fn default_work_dir() -> PathBuf {
    PathBuf::from("worklogs/items")
}

fn default_templates_dir() -> PathBuf {
    PathBuf::from("templates")
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            spec_root: default_spec_root(),
            rfc_output: default_rfc_output(),
            adr_dir: default_adr_dir(),
            work_dir: default_work_dir(),
            templates_dir: default_templates_dir(),
        }
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

impl Config {
    /// Load config from file or use defaults
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path
            .map(PathBuf::from)
            .or_else(Self::find_config)
            .unwrap_or_else(|| PathBuf::from("phaseos.toml"));

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
            let config_path = current.join("phaseos.toml");
            if config_path.exists() {
                return Some(config_path);
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Get the RFC spec directory (spec/rfcs/)
    pub fn rfcs_dir(&self) -> PathBuf {
        self.paths.spec_root.join("rfcs")
    }

    /// Get the schema directory (spec/schema/)
    pub fn schema_dir(&self) -> PathBuf {
        self.paths.spec_root.join("schema")
    }

    /// Generate default config TOML
    pub fn default_toml() -> &'static str {
        r#"[project]
name = "my-project"

[paths]
spec_root = "spec"
rfc_output = "docs/rfc"
adr_dir = "docs/adr"
work_dir = "worklogs/items"
templates_dir = "templates"

[schema]
version = 1
"#
    }
}

