use super::LoadError;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{ClauseEntry, ClauseWire, RfcIndex, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_toml_value};
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Load all RFCs from the gov/rfc directory
pub fn load_rfcs(config: &Config) -> Result<Vec<RfcIndex>, LoadError> {
    let rfcs_dir = config.rfc_dir();
    if !rfcs_dir.exists() {
        return Ok(vec![]);
    }

    let mut rfcs = Vec::new();
    let entries = std::fs::read_dir(&rfcs_dir).map_err(|e| LoadError::Io {
        file: rfcs_dir.display().to_string(),
        action: "read RFC directory",
        message: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| LoadError::Io {
            file: rfcs_dir.display().to_string(),
            action: "read RFC directory entry",
            message: e.to_string(),
        })?;

        let path = entry.path();
        if path.is_dir() {
            reject_legacy_json_in_rfc_dir(config, &path).map_err(LoadError::Diagnostic)?;
        }
        if path.is_dir()
            && let Some(rfc_path) = find_rfc_in_dir(&path)
        {
            let rfc_index = load_rfc(config, &rfc_path)?;
            rfcs.push(rfc_index);
        }
    }

    rfcs.sort_by(|a, b| a.rfc.rfc_id.cmp(&b.rfc.rfc_id));

    Ok(rfcs)
}

/// Load a single RFC and its clauses
pub fn load_rfc(config: &Config, rfc_path: &Path) -> Result<RfcIndex, LoadError> {
    if rfc_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        return Err(LoadError::Diagnostic(legacy_json_diagnostic(
            config, rfc_path,
        )));
    }

    let rfc_dir = rfc_path.parent().ok_or_else(|| LoadError::InternalIo {
        file: rfc_path.display().to_string(),
        message: "RFC path has no parent directory".to_string(),
    })?;
    let resolved_rfc_root = canonicalize_path(&config.rfc_dir(), "resolve RFC storage root")?;
    let resolved_rfc_dir = canonicalize_path(rfc_dir, "resolve RFC directory")?;
    ensure_path_contained(
        &resolved_rfc_root,
        &resolved_rfc_dir,
        rfc_path,
        &rfc_dir.display().to_string(),
    )?;
    let resolved_rfc_path = canonicalize_path(rfc_path, "resolve RFC path")?;
    ensure_path_contained(
        &resolved_rfc_dir,
        &resolved_rfc_path,
        rfc_path,
        &rfc_path.display().to_string(),
    )?;

    let rfc: RfcSpec = load_source_wire::<RfcWire>(
        config,
        rfc_path,
        SourceWireSpec {
            read_action: "read RFC",
            schema: ArtifactSchema::Rfc,
            schema_error: rfc_schema_error,
            decode_error: json_error,
        },
    )?
    .into();

    reject_legacy_json_in_rfc_dir(config, rfc_dir).map_err(LoadError::Diagnostic)?;

    let mut clause_paths = BTreeSet::new();
    for section in &rfc.sections {
        for clause_path in &section.clauses {
            if !is_safe_relative_clause_path(clause_path) {
                return Err(LoadError::ClausePathInvalid {
                    file: rfc_path.display().to_string(),
                    clause: clause_path.clone(),
                });
            }
            let full_path = rfc_dir.join(clause_path);
            let exists = full_path.try_exists().map_err(|err| LoadError::Io {
                file: full_path.display().to_string(),
                action: "inspect clause path",
                message: err.to_string(),
            })?;
            if !exists || full_path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                return Err(LoadError::ClausePathInvalid {
                    file: rfc_path.display().to_string(),
                    clause: clause_path.clone(),
                });
            }
            let resolved_clause = canonicalize_path(&full_path, "resolve clause path")?;
            if !resolved_clause.is_file() {
                return Err(LoadError::ClausePathInvalid {
                    file: rfc_path.display().to_string(),
                    clause: clause_path.clone(),
                });
            }
            ensure_path_contained(&resolved_rfc_dir, &resolved_clause, rfc_path, clause_path)?;
            clause_paths.insert(full_path);
        }
    }
    clause_paths.extend(clause_directory_paths(
        rfc_dir,
        &resolved_rfc_dir,
        rfc_path,
    )?);
    let clauses = clause_paths
        .into_iter()
        .map(|path| super::load_clause(config, &path))
        .collect::<Result<_, _>>()?;

    Ok(RfcIndex {
        rfc,
        clauses,
        path: rfc_path.to_path_buf(),
    })
}

fn is_safe_relative_clause_path(clause_path: &str) -> bool {
    let path = Path::new(clause_path);
    !path.as_os_str().is_empty() && !path.is_absolute()
}

fn canonicalize_path(path: &Path, action: &'static str) -> Result<PathBuf, LoadError> {
    std::fs::canonicalize(path).map_err(|err| LoadError::Io {
        file: path.display().to_string(),
        action,
        message: err.to_string(),
    })
}

fn ensure_path_contained(
    resolved_root: &Path,
    resolved_path: &Path,
    rfc_path: &Path,
    clause_reference: &str,
) -> Result<(), LoadError> {
    if !resolved_path.starts_with(resolved_root) {
        return Err(LoadError::ClausePathInvalid {
            file: rfc_path.display().to_string(),
            clause: clause_reference.to_string(),
        });
    }
    Ok(())
}

fn clause_directory_paths(
    rfc_dir: &Path,
    resolved_rfc_dir: &Path,
    rfc_path: &Path,
) -> Result<Vec<PathBuf>, LoadError> {
    let clauses_dir = rfc_dir.join("clauses");
    if !clauses_dir.exists() {
        return Ok(vec![]);
    }
    let resolved_clauses_dir = canonicalize_path(&clauses_dir, "resolve clause directory")?;
    ensure_path_contained(
        resolved_rfc_dir,
        &resolved_clauses_dir,
        rfc_path,
        &clauses_dir.display().to_string(),
    )?;

    let entries = std::fs::read_dir(&clauses_dir).map_err(|err| LoadError::Io {
        file: clauses_dir.display().to_string(),
        action: "read clause directory",
        message: err.to_string(),
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| LoadError::Io {
            file: clauses_dir.display().to_string(),
            action: "read clause directory entry",
            message: err.to_string(),
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            let resolved_clause = canonicalize_path(&path, "resolve clause path")?;
            ensure_path_contained(
                resolved_rfc_dir,
                &resolved_clause,
                rfc_path,
                &path.display().to_string(),
            )?;
            paths.push(path);
        }
    }
    paths.sort();

    Ok(paths)
}

/// Load a single clause
pub(super) fn load_clause_file(config: &Config, path: &Path) -> Result<ClauseEntry, LoadError> {
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        return Err(LoadError::Diagnostic(legacy_json_diagnostic(config, path)));
    }

    let spec = load_source_wire::<ClauseWire>(
        config,
        path,
        SourceWireSpec {
            read_action: "read clause",
            schema: ArtifactSchema::Clause,
            schema_error: clause_schema_error,
            decode_error: clause_schema_error,
        },
    )?
    .into();

    Ok(ClauseEntry {
        spec,
        path: path.to_path_buf(),
    })
}

pub fn find_rfc_toml(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let path = config.rfc_source_path(rfc_id, "toml");
    path.exists().then_some(path)
}

pub fn find_clause_toml(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let (rfc_id, clause_name) = split_clause_id(clause_id)?;
    let clause_path = config.clause_source_path(rfc_id, clause_name, "toml");
    clause_path.exists().then_some(clause_path)
}

pub fn reject_legacy_json_storage(config: &Config) -> DiagnosticResult<()> {
    let rfc_root = config.rfc_dir();
    if !rfc_root.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(&rfc_root).map_err(|err| {
        Diagnostic::io_error(
            "read RFC directory for legacy JSON scan",
            err,
            config.display_path(&rfc_root).display().to_string(),
        )
    })?;
    let mut dirs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::io_error(
                "read RFC directory entry for legacy JSON scan",
                err,
                config.display_path(&rfc_root).display().to_string(),
            )
        })?;
        if entry.path().is_dir() {
            dirs.push(entry.path());
        }
    }
    dirs.sort();

    for dir in dirs {
        reject_legacy_json_in_rfc_dir(config, &dir)?;
    }
    Ok(())
}

fn reject_legacy_json_in_rfc_dir(config: &Config, rfc_dir: &Path) -> DiagnosticResult<()> {
    let rfc_json = rfc_dir.join("rfc.json");
    if rfc_json.exists() {
        return Err(legacy_json_diagnostic(config, &rfc_json));
    }

    let clauses_dir = rfc_dir.join("clauses");
    if !clauses_dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(&clauses_dir).map_err(|err| {
        Diagnostic::io_error(
            "read clause directory for legacy JSON scan",
            err,
            config.display_path(&clauses_dir).display().to_string(),
        )
    })?;
    let mut clauses = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::io_error(
                "read clause directory entry for legacy JSON scan",
                err,
                config.display_path(&clauses_dir).display().to_string(),
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            clauses.push(path);
        }
    }
    clauses.sort();

    if let Some(path) = clauses.first() {
        return Err(legacy_json_diagnostic(config, path));
    }
    Ok(())
}

fn legacy_json_diagnostic(config: &Config, path: &Path) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0505MigrationRequired,
        "Legacy RFC/clause JSON artifact storage is unsupported. Migrate this repository with a compatible earlier govctl version before upgrading.",
        config.display_path(path).display().to_string(),
    )
}

fn read_source_file(path: &Path, action: &'static str) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        file: path.display().to_string(),
        action,
        message: e.to_string(),
    })
}

struct SourceWireSpec {
    read_action: &'static str,
    schema: ArtifactSchema,
    schema_error: fn(String, String) -> LoadError,
    decode_error: fn(String, String) -> LoadError,
}

fn load_source_wire<Wire>(
    config: &Config,
    path: &Path,
    spec: SourceWireSpec,
) -> Result<Wire, LoadError>
where
    Wire: DeserializeOwned,
{
    let content = read_source_file(path, spec.read_action)?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => load_toml_wire(config, path, &content, spec),
        Some("json") => Err(LoadError::Diagnostic(legacy_json_diagnostic(config, path))),
        _ => Err((spec.schema_error)(
            path.display().to_string(),
            "Unsupported artifact source extension; expected TOML".to_string(),
        )),
    }
}

fn load_toml_wire<Wire>(
    config: &Config,
    path: &Path,
    content: &str,
    spec: SourceWireSpec,
) -> Result<Wire, LoadError>
where
    Wire: DeserializeOwned,
{
    let raw: toml::Value = toml::from_str(content)
        .map_err(|e| (spec.decode_error)(path.display().to_string(), e.to_string()))?;
    validate_toml_value(spec.schema, config, path, &raw)
        .map_err(|e| (spec.schema_error)(path.display().to_string(), e.message))?;
    raw.try_into()
        .map_err(|e| (spec.decode_error)(path.display().to_string(), e.to_string()))
}

fn rfc_schema_error(file: String, message: String) -> LoadError {
    LoadError::RfcSchema { file, message }
}

fn json_error(file: String, message: String) -> LoadError {
    LoadError::Json { file, message }
}

fn clause_schema_error(file: String, message: String) -> LoadError {
    LoadError::ClauseSchema { file, message }
}

pub(crate) fn split_clause_id(clause_id: &str) -> Option<(&str, &str)> {
    let mut parts = clause_id.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(rfc_id), Some(clause_name), None)
            if valid_rfc_id(rfc_id) && valid_clause_name(clause_name) =>
        {
            Some((rfc_id, clause_name))
        }
        _ => None,
    }
}

fn valid_rfc_id(id: &str) -> bool {
    id.strip_prefix("RFC-")
        .is_some_and(|suffix| suffix.len() == 4 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
}

fn valid_clause_name(id: &str) -> bool {
    id.strip_prefix("C-").is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
    })
}

fn find_rfc_in_dir(dir: &Path) -> Option<PathBuf> {
    let toml = dir.join("rfc.toml");
    toml.exists().then_some(toml)
}

#[cfg(test)]
mod tests {
    use super::is_safe_relative_clause_path;

    #[test]
    fn clause_paths_require_relative_syntax() {
        for path in [
            "clauses/C-ONE.toml",
            "C-ALT.toml",
            "./clauses/C-ONE.toml",
            "../C-ONE.toml",
            "clauses//C-ONE.toml",
        ] {
            assert!(is_safe_relative_clause_path(path), "{path}");
        }
        assert!(!is_safe_relative_clause_path(""));

        let absolute = std::env::temp_dir().join("C-ONE.toml");
        assert!(
            !is_safe_relative_clause_path(&absolute.display().to_string()),
            "{}",
            absolute.display()
        );
    }
}
