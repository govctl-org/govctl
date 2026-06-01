use super::LoadError;
use crate::config::Config;
use crate::model::{ClauseEntry, ClauseWire, RfcIndex, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_json_value, validate_toml_value};
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
    let content = read_source_file(rfc_path, "read RFC")?;

    let rfc: RfcSpec = match rfc_path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            let mut raw: toml::Value = toml::from_str(&content).map_err(|e| LoadError::Json {
                file: rfc_path.display().to_string(),
                message: e.to_string(),
            })?;
            crate::write::normalize_rfc_value(&mut raw);
            validate_toml_value(ArtifactSchema::Rfc, config, rfc_path, &raw).map_err(|e| {
                LoadError::RfcSchema {
                    file: rfc_path.display().to_string(),
                    message: e.message,
                }
            })?;
            let wire: RfcWire = raw.try_into().map_err(|e| LoadError::Json {
                file: rfc_path.display().to_string(),
                message: e.to_string(),
            })?;
            wire.into()
        }
        _ => {
            let mut raw: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| LoadError::Json {
                    file: rfc_path.display().to_string(),
                    message: e.to_string(),
                })?;
            crate::write::normalize_rfc_json(&mut raw);
            validate_json_value(ArtifactSchema::Rfc, config, rfc_path, &raw).map_err(|e| {
                LoadError::RfcSchema {
                    file: rfc_path.display().to_string(),
                    message: e.message,
                }
            })?;
            let wire: RfcWire = serde_json::from_value(raw).map_err(|e| LoadError::Json {
                file: rfc_path.display().to_string(),
                message: e.to_string(),
            })?;
            wire.into()
        }
    };

    let rfc_dir = rfc_path.parent().ok_or_else(|| LoadError::InternalIo {
        file: rfc_path.display().to_string(),
        message: "RFC path has no parent directory".to_string(),
    })?;
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
    let content = read_source_file(path, "read clause")?;

    let spec = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            let mut raw: toml::Value = toml::from_str(&content).map_err(|e| LoadError::Json {
                file: path.display().to_string(),
                message: e.to_string(),
            })?;
            crate::write::normalize_clause_value(&mut raw);
            validate_toml_value(ArtifactSchema::Clause, config, path, &raw).map_err(|e| {
                LoadError::ClauseSchema {
                    file: path.display().to_string(),
                    message: e.message,
                }
            })?;
            let wire: ClauseWire = raw.try_into().map_err(|e| LoadError::Json {
                file: path.display().to_string(),
                message: e.to_string(),
            })?;
            let spec: crate::model::ClauseSpec = wire.into();
            spec
        }
        _ => {
            let mut raw: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| LoadError::Json {
                    file: path.display().to_string(),
                    message: e.to_string(),
                })?;
            crate::write::normalize_clause_json(&mut raw);
            validate_json_value(ArtifactSchema::Clause, config, path, &raw).map_err(|e| {
                LoadError::ClauseSchema {
                    file: path.display().to_string(),
                    message: e.message,
                }
            })?;
            let wire: ClauseWire = serde_json::from_value(raw).map_err(|e| LoadError::Json {
                file: path.display().to_string(),
                message: e.to_string(),
            })?;
            wire.into()
        }
    };

    Ok(ClauseEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Find an RFC source file by ID, preferring TOML over legacy JSON.
pub fn find_rfc_json(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let rfc_dir = config.rfc_dir().join(rfc_id);
    find_rfc_in_dir(&rfc_dir)
}

pub fn find_rfc_toml(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let path = config.rfc_dir().join(rfc_id).join("rfc.toml");
    path.exists().then_some(path)
}

/// Find a clause source file by full ID, preferring TOML over legacy JSON.
pub fn find_clause_json(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let (rfc_id, clause_name) = split_clause_id(clause_id)?;
    let clause_path = clause_source_path(config, rfc_id, clause_name, "toml");

    if clause_path.exists() {
        Some(clause_path)
    } else {
        let legacy_clause_path = clause_source_path(config, rfc_id, clause_name, "json");
        legacy_clause_path.exists().then_some(legacy_clause_path)
    }
}

pub fn find_clause_toml(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let (rfc_id, clause_name) = split_clause_id(clause_id)?;
    let clause_path = clause_source_path(config, rfc_id, clause_name, "toml");
    clause_path.exists().then_some(clause_path)
}

fn read_source_file(path: &Path, action: &'static str) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        file: path.display().to_string(),
        action,
        message: e.to_string(),
    })
}

pub(crate) fn split_clause_id(clause_id: &str) -> Option<(&str, &str)> {
    let mut parts = clause_id.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(rfc_id), Some(clause_name), None) => Some((rfc_id, clause_name)),
        _ => None,
    }
}

fn clause_source_path(
    config: &Config,
    rfc_id: &str,
    clause_name: &str,
    extension: &str,
) -> PathBuf {
    config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.{extension}"))
}

fn find_rfc_in_dir(dir: &Path) -> Option<PathBuf> {
    let toml = dir.join("rfc.toml");
    if toml.exists() {
        return Some(toml);
    }
    let json = dir.join("rfc.json");
    json.exists().then_some(json)
}
