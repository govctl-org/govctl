//! Configuration loading and management.

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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default = "default_project_name")]
    pub name: String,
}

fn default_project_name() -> String {
    "govctl-project".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Root directory for governance SSOT (gov/)
    #[serde(default = "default_gov_root")]
    pub gov_root: PathBuf,
    /// Output directory for rendered docs (docs/)
    #[serde(default = "default_docs_output")]
    pub docs_output: PathBuf,
}

fn default_gov_root() -> PathBuf {
    PathBuf::from("gov")
}

fn default_docs_output() -> PathBuf {
    PathBuf::from("docs")
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            gov_root: default_gov_root(),
            docs_output: default_docs_output(),
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

    /// Generate default config TOML
    pub fn default_toml() -> &'static str {
        r#"[project]
name = "my-project"

[paths]
gov_root = "gov"
docs_output = "docs"

[schema]
version = 1
"#
    }
}
