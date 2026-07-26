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
    let resolved_rfc_dir = canonicalize_contained(
        rfc_dir,
        &resolved_rfc_root,
        rfc_path,
        &rfc_dir.display().to_string(),
        "resolve RFC directory",
    )?;
    canonicalize_contained(
        rfc_path,
        &resolved_rfc_dir,
        rfc_path,
        &rfc_path.display().to_string(),
        "resolve RFC path",
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
        for clause_reference in &section.clauses {
            clause_paths.insert(resolve_clause_reference(
                rfc_dir,
                &resolved_rfc_dir,
                rfc_path,
                clause_reference,
            )?);
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

fn resolve_clause_reference(
    rfc_dir: &Path,
    resolved_rfc_dir: &Path,
    rfc_path: &Path,
    reference: &str,
) -> Result<PathBuf, LoadError> {
    let path = Path::new(reference);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.extension().and_then(|extension| extension.to_str()) != Some("toml")
    {
        return Err(invalid_clause_path(rfc_path, reference));
    }

    let path = rfc_dir.join(path);
    let resolved = std::fs::canonicalize(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            invalid_clause_path(rfc_path, reference)
        } else {
            LoadError::Io {
                file: path.display().to_string(),
                action: "resolve clause path",
                message: err.to_string(),
            }
        }
    })?;
    if !resolved.is_file() || !resolved.starts_with(resolved_rfc_dir) {
        return Err(invalid_clause_path(rfc_path, reference));
    }
    Ok(path)
}

fn invalid_clause_path(rfc_path: &Path, reference: &str) -> LoadError {
    LoadError::ClausePathInvalid {
        file: rfc_path.display().to_string(),
        clause: reference.to_string(),
    }
}

fn canonicalize_path(path: &Path, action: &'static str) -> Result<PathBuf, LoadError> {
    std::fs::canonicalize(path).map_err(|err| LoadError::Io {
        file: path.display().to_string(),
        action,
        message: err.to_string(),
    })
}

fn canonicalize_contained(
    path: &Path,
    root: &Path,
    rfc_path: &Path,
    reference: &str,
    action: &'static str,
) -> Result<PathBuf, LoadError> {
    let resolved = canonicalize_path(path, action)?;
    if !resolved.starts_with(root) {
        return Err(invalid_clause_path(rfc_path, reference));
    }
    Ok(resolved)
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
    canonicalize_contained(
        &clauses_dir,
        resolved_rfc_dir,
        rfc_path,
        &clauses_dir.display().to_string(),
        "resolve clause directory",
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
            canonicalize_contained(
                &path,
                resolved_rfc_dir,
                rfc_path,
                &path.display().to_string(),
                "resolve clause path",
            )?;
            paths.push(path);
        }
    }
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
