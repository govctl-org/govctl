use super::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use std::path::{Path, PathBuf};

impl Config {
    /// Load config from file or use defaults
    ///
    /// All relative paths in the config are resolved relative to the project root
    /// (the parent of gov/config.toml), not the current working directory.
    pub fn load(path: Option<&Path>) -> DiagnosticResult<Self> {
        Self::load_inner(path, false)
    }

    pub fn load_for_migration(path: Option<&Path>) -> DiagnosticResult<Self> {
        Self::load_inner(path, true)
    }

    fn load_inner(path: Option<&Path>, allow_outdated_schema: bool) -> DiagnosticResult<Self> {
        let config_path = if let Some(path) = path {
            let path = PathBuf::from(path);
            if !path_entry_exists(&path)? {
                return Err(explicit_config_not_found_diagnostic(&path));
            }
            path
        } else {
            Self::find_config()?.unwrap_or_else(|| PathBuf::from("gov/config.toml"))
        };

        if path_entry_exists(&config_path)? {
            let content = std::fs::read_to_string(&config_path).map_err(|err| {
                Diagnostic::io_error("read config", err, config_path.display().to_string())
            })?;
            let raw: toml::Value = toml::from_str(&content).map_err(|err| {
                Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    format!("Failed to parse config: {err}"),
                    config_path.display().to_string(),
                )
            })?;
            let schema_version = raw
                .get("schema")
                .and_then(toml::Value::as_table)
                .and_then(|schema| schema.get("version"))
                .and_then(toml::Value::as_integer)
                .and_then(|version| u32::try_from(version).ok())
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::E0501ConfigInvalid,
                        "Missing or invalid required config field: schema.version",
                        config_path.display().to_string(),
                    )
                })?;
            crate::cmd::migrate::validate_supported_schema_version(
                schema_version,
                config_path.display().to_string(),
            )?;
            if schema_version == 4 && !allow_outdated_schema {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0505MigrationRequired,
                    "Schema version 4 is outdated. Run `govctl migrate` to upgrade to schema version 5.",
                    config_path.display().to_string(),
                ));
            }
            let mut config: Config = raw.try_into().map_err(|err| {
                Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    format!("Failed to parse config: {err}"),
                    config_path.display().to_string(),
                )
            })?;
            if schema_version >= 5 && config.source_scan.legacy_exclude.is_some() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    "Unsupported config field for schema version 5: source_scan.exclude",
                    config_path.display().to_string(),
                ));
            }

            resolve_project_paths(&mut config, &config_path);

            Ok(config)
        } else {
            let mut config = Config::default();
            resolve_project_paths(&mut config, &config_path);
            Ok(config)
        }
    }

    pub fn for_init(force: bool) -> DiagnosticResult<Self> {
        let project_root = std::env::current_dir()
            .map_err(|err| Diagnostic::io_error("resolve current directory", err, "."))?;
        let config_path = project_root.join("gov/config.toml");
        if path_entry_exists(&config_path)? {
            return Self::load(Some(&config_path));
        }
        if !force && contains_governance_state(&project_root.join("gov"))? {
            return Err(missing_config_diagnostic(&config_path));
        }
        let mut config = Self::default();
        resolve_project_paths(&mut config, &config_path);
        Ok(config)
    }

    /// Find a config file by walking up the directory tree.
    fn find_config() -> DiagnosticResult<Option<PathBuf>> {
        let mut current = std::env::current_dir()
            .map_err(|err| Diagnostic::io_error("resolve current directory", err, "."))?;
        loop {
            let config_path = current.join("gov/config.toml");
            if path_entry_exists(&config_path)? {
                return Ok(Some(config_path));
            }
            if contains_governance_state(&current.join("gov"))? {
                return Err(missing_config_diagnostic(&config_path));
            }
            if !current.pop() {
                return Ok(None);
            }
        }
    }

    /// Project root directory, derived as the parent of `gov/`.
    pub fn project_root(&self) -> &Path {
        self.gov_root
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    }

    pub fn rfc_dir(&self) -> PathBuf {
        self.gov_root.join("rfc")
    }

    pub fn rfc_artifact_dir(&self, rfc_id: &str) -> PathBuf {
        self.rfc_dir().join(rfc_id)
    }

    pub fn rfc_source_path(&self, rfc_id: &str, extension: &str) -> PathBuf {
        self.rfc_artifact_dir(rfc_id)
            .join(format!("rfc.{extension}"))
    }

    pub fn clause_dir(&self, rfc_id: &str) -> PathBuf {
        self.rfc_artifact_dir(rfc_id).join("clauses")
    }

    pub fn clause_source_path(&self, rfc_id: &str, clause_name: &str, extension: &str) -> PathBuf {
        self.clause_dir(rfc_id)
            .join(format!("{clause_name}.{extension}"))
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

    pub fn conformance_dir(&self) -> PathBuf {
        self.gov_root.join("conformance")
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
        path.strip_prefix(self.project_root())
            .map(PathBuf::from)
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn missing_config_diagnostic(config_path: &Path) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0505MigrationRequired,
        "gov/config.toml is missing for an existing governance project. Restore the project configuration before using govctl.",
        config_path.display().to_string(),
    )
}

fn explicit_config_not_found_diagnostic(config_path: &Path) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0502PathNotFound,
        format!("Configuration file not found: {}", config_path.display()),
        config_path.display().to_string(),
    )
}

fn contains_governance_state(gov_root: &Path) -> DiagnosticResult<bool> {
    if path_entry_exists(&gov_root.join("releases.toml"))? {
        return Ok(true);
    }

    let mut pending = Vec::new();
    for name in [
        "rfc",
        "adr",
        "work",
        "guard",
        "conformance",
        "schema",
        "templates",
    ] {
        let path = gov_root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(true),
            Ok(metadata) if metadata.is_dir() => pending.push(path),
            Ok(_) => return Ok(true),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(Diagnostic::io_error(
                    "inspect governance path",
                    err,
                    path.display().to_string(),
                ));
            }
        }
    }
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|err| {
            Diagnostic::io_error(
                "inspect governance artifact directory",
                err,
                dir.display().to_string(),
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                Diagnostic::io_error(
                    "inspect governance artifact directory entry",
                    err,
                    dir.display().to_string(),
                )
            })?;
            let file_type = entry.file_type().map_err(|err| {
                Diagnostic::io_error(
                    "inspect governance artifact type",
                    err,
                    entry.path().display().to_string(),
                )
            })?;
            if file_type.is_symlink() {
                return Ok(true);
            } else if file_type.is_dir() {
                pending.push(entry.path());
            } else if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "toml" || extension == "json")
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn path_entry_exists(path: &Path) -> DiagnosticResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(Diagnostic::io_error(
            "inspect path",
            err,
            path.display().to_string(),
        )),
    }
}

fn resolve_project_paths(config: &mut Config, config_path: &Path) {
    let Some(project_root) = config_path.parent().and_then(|path| path.parent()) else {
        return;
    };
    config.gov_root = project_root.join("gov");
    if config.paths.docs_output.is_relative() {
        config.paths.docs_output = project_root.join(&config.paths.docs_output);
    }
    if config.paths.agent_dir.is_relative() {
        config.paths.agent_dir = project_root.join(&config.paths.agent_dir);
    }
}
