use super::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use anyhow::Result;
use std::path::{Path, PathBuf};

impl Config {
    /// Load config from file or use defaults
    ///
    /// All relative paths in the config are resolved relative to the project root
    /// (the parent of gov/config.toml), not the current working directory.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path
            .map(PathBuf::from)
            .or_else(Self::find_config)
            .unwrap_or_else(|| PathBuf::from("gov/config.toml"));

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|err| {
                Diagnostic::io_error("read config", err, config_path.display().to_string())
            })?;
            let mut config: Config = toml::from_str(&content).map_err(|err| {
                Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    format!("Failed to parse config: {err}"),
                    config_path.display().to_string(),
                )
            })?;

            // Resolve paths to absolute. gov_root is always <project_root>/gov.
            if let Some(project_root) = config_path.parent().and_then(|p| p.parent()) {
                config.gov_root = project_root.join("gov");
                if config.paths.docs_output.is_relative() {
                    config.paths.docs_output = project_root.join(&config.paths.docs_output);
                }
                if config.paths.agent_dir.is_relative() {
                    config.paths.agent_dir = project_root.join(&config.paths.agent_dir);
                }
            }

            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Find config file by walking up directory tree.
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

    pub fn rfc_dir(&self) -> PathBuf {
        self.gov_root.join("rfc")
    }

    pub fn adr_dir(&self) -> PathBuf {
        self.gov_root.join("adr")
    }

    pub fn work_dir(&self) -> PathBuf {
        self.gov_root.join("work")
    }

    pub fn schema_dir(&self) -> PathBuf {
        self.gov_root.join("schema")
    }

    pub fn guard_dir(&self) -> PathBuf {
        self.gov_root.join("guard")
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.gov_root.join("templates")
    }

    pub fn rfc_output(&self) -> PathBuf {
        self.paths.docs_output.join("rfc")
    }

    pub fn adr_output(&self) -> PathBuf {
        self.paths.docs_output.join("adr")
    }

    pub fn work_output(&self) -> PathBuf {
        self.paths.docs_output.join("work")
    }

    pub fn releases_path(&self) -> PathBuf {
        self.gov_root.join("releases.toml")
    }

    /// Path for user-facing display: relative to project root when under it.
    pub fn display_path(&self, path: &Path) -> PathBuf {
        self.gov_root
            .parent()
            .and_then(|root| path.strip_prefix(root).ok())
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf())
    }
}
