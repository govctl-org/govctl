use super::LoadError;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{ClauseEntry, ClauseWire, RfcIndex, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_toml_value};
use serde::de::DeserializeOwned;
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

    let rfc: RfcSpec = load_source_wire::<RfcWire>(
        config,
        rfc_path,
        SourceWireSpec {
            read_action: "read RFC",
            schema: ArtifactSchema::Rfc,
            normalize_toml: crate::write::normalize_rfc_value,
            schema_error: rfc_schema_error,
        },
    )?
    .into();

    let rfc_dir = rfc_path.parent().ok_or_else(|| LoadError::InternalIo {
        file: rfc_path.display().to_string(),
        message: "RFC path has no parent directory".to_string(),
    })?;
    reject_legacy_json_in_rfc_dir(config, rfc_dir).map_err(LoadError::Diagnostic)?;
    let mut clauses = Vec::new();

    for section in &rfc.sections {
        for clause_path in &section.clauses {
            if clause_path.contains("..") {
                return Err(LoadError::ClausePathInvalid {
                    file: rfc_path.display().to_string(),
                    clause: clause_path.clone(),
                });
            }

            let full_path = rfc_dir.join(clause_path);
            if full_path.exists() {
                let clause = super::load_clause(config, &full_path)?;
                clauses.push(clause);
            }
        }
    }

    Ok(RfcIndex {
        rfc,
        clauses,
        path: rfc_path.to_path_buf(),
    })
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
            normalize_toml: crate::write::normalize_clause_value,
            schema_error: clause_schema_error,
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

    let mut dirs: Vec<_> = std::fs::read_dir(&rfc_root)
        .map_err(|err| {
            Diagnostic::io_error(
                "read RFC directory for legacy JSON scan",
                err,
                config.display_path(&rfc_root).display().to_string(),
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
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

    let mut clauses: Vec<_> = std::fs::read_dir(&clauses_dir)
        .map_err(|err| {
            Diagnostic::io_error(
                "read clause directory for legacy JSON scan",
                err,
                config.display_path(&clauses_dir).display().to_string(),
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    clauses.sort();

    if let Some(path) = clauses.first() {
        return Err(legacy_json_diagnostic(config, path));
    }
    Ok(())
}

fn legacy_json_diagnostic(config: &Config, path: &Path) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0505MigrationRequired,
        "Legacy RFC/clause JSON artifact storage is no longer supported. Use govctl <0.9 to run `govctl migrate` before upgrading.",
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
    normalize_toml: fn(&mut toml::Value),
    schema_error: fn(String, String) -> LoadError,
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
    let mut raw: toml::Value = toml::from_str(content).map_err(|e| LoadError::Json {
        file: path.display().to_string(),
        message: e.to_string(),
    })?;
    (spec.normalize_toml)(&mut raw);
    validate_toml_value(spec.schema, config, path, &raw)
        .map_err(|e| (spec.schema_error)(path.display().to_string(), e.message))?;
    raw.try_into().map_err(|e| LoadError::Json {
        file: path.display().to_string(),
        message: e.to_string(),
    })
}

fn rfc_schema_error(file: String, message: String) -> LoadError {
    LoadError::RfcSchema { file, message }
}

fn clause_schema_error(file: String, message: String) -> LoadError {
    LoadError::ClauseSchema { file, message }
}

pub(crate) fn split_clause_id(clause_id: &str) -> Option<(&str, &str)> {
    let mut parts = clause_id.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(rfc_id), Some(clause_name), None) => Some((rfc_id, clause_name)),
        _ => None,
    }
}

fn find_rfc_in_dir(dir: &Path) -> Option<PathBuf> {
    let toml = dir.join("rfc.toml");
    toml.exists().then_some(toml)
}
