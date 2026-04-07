//! JSON loading for RFC and clause files.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseEntry, ClauseWire, ProjectIndex, RfcIndex, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_json_value, validate_toml_value};
use std::path::{Path, PathBuf};

/// Load error types
#[derive(Debug)]
#[allow(dead_code)]
pub enum LoadError {
    Io { file: String, message: String },
    Json { file: String, message: String },
    RfcSchema { file: String, message: String },
    ClauseSchema { file: String, message: String },
    ClausePathInvalid { file: String, clause: String },
}

impl From<LoadError> for Diagnostic {
    fn from(err: LoadError) -> Self {
        match err {
            LoadError::Io { file, message } => {
                Diagnostic::new(DiagnosticCode::E0901IoError, message, file)
            }
            LoadError::Json { file, message } => {
                Diagnostic::new(DiagnosticCode::E0902JsonParseError, message, file)
            }
            LoadError::RfcSchema { file, message } => {
                Diagnostic::new(DiagnosticCode::E0101RfcSchemaInvalid, message, file)
            }
            LoadError::ClauseSchema { file, message } => {
                Diagnostic::new(DiagnosticCode::E0201ClauseSchemaInvalid, message, file)
            }
            LoadError::ClausePathInvalid { file, clause } => Diagnostic::new(
                DiagnosticCode::E0204ClausePathInvalid,
                format!("Invalid clause path: {clause}"),
                file,
            ),
        }
    }
}

/// Load all RFCs from the gov/rfc directory
pub fn load_rfcs(config: &Config) -> Result<Vec<RfcIndex>, LoadError> {
    let rfcs_dir = config.rfc_dir();
    if !rfcs_dir.exists() {
        return Ok(vec![]);
    }

    let mut rfcs = Vec::new();
    let entries = std::fs::read_dir(&rfcs_dir).map_err(|e| LoadError::Io {
        file: rfcs_dir.display().to_string(),
        message: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| LoadError::Io {
            file: rfcs_dir.display().to_string(),
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

    // Sort by RFC ID for deterministic output
    rfcs.sort_by(|a, b| a.rfc.rfc_id.cmp(&b.rfc.rfc_id));

    Ok(rfcs)
}

/// Load a single RFC and its clauses
pub fn load_rfc(config: &Config, rfc_path: &Path) -> Result<RfcIndex, LoadError> {
    let content = std::fs::read_to_string(rfc_path).map_err(|e| LoadError::Io {
        file: rfc_path.display().to_string(),
        message: e.to_string(),
    })?;

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

    let rfc_dir = rfc_path.parent().ok_or_else(|| LoadError::Io {
        file: rfc_path.display().to_string(),
        message: "RFC path has no parent directory".to_string(),
    })?;
    let mut clauses = Vec::new();

    // Load all clauses referenced in sections
    for section in &rfc.sections {
        for clause_path in &section.clauses {
            // Validate path doesn't escape
            if clause_path.contains("..") {
                return Err(LoadError::ClausePathInvalid {
                    file: rfc_path.display().to_string(),
                    clause: clause_path.clone(),
                });
            }

            let full_path = rfc_dir.join(clause_path);
            if full_path.exists() {
                let clause = load_clause(config, &full_path)?;
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
pub fn load_clause(config: &Config, path: &Path) -> Result<ClauseEntry, LoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        file: path.display().to_string(),
        message: e.to_string(),
    })?;

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

fn find_rfc_in_dir(dir: &Path) -> Option<PathBuf> {
    let toml = dir.join("rfc.toml");
    if toml.exists() {
        return Some(toml);
    }
    let json = dir.join("rfc.json");
    json.exists().then_some(json)
}

/// Result of loading a project: index plus any warnings encountered
pub struct ProjectLoadResult {
    pub index: ProjectIndex,
    pub warnings: Vec<Diagnostic>,
}

/// Load full project index (RFCs, ADRs, Work Items)
pub fn load_project(config: &Config) -> Result<ProjectIndex, Vec<Diagnostic>> {
    load_project_with_warnings(config).map(|r| r.index)
}

/// Load full project index, returning both the index and any parse warnings
pub fn load_project_with_warnings(config: &Config) -> Result<ProjectLoadResult, Vec<Diagnostic>> {
    let mut index = ProjectIndex::default();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Load RFCs
    match load_rfcs(config) {
        Ok(rfcs) => index.rfcs = rfcs,
        Err(e) => errors.push(e.into()),
    }

    // Load ADRs (with warnings for skipped files)
    match crate::parse::load_adrs_with_warnings(config) {
        Ok(result) => {
            index.adrs = result.items;
            warnings.extend(result.warnings);
        }
        Err(e) => errors.push(e),
    }

    // Load Work Items (with warnings for skipped files)
    match crate::parse::load_work_items_with_warnings(config) {
        Ok(result) => {
            index.work_items = result.items;
            warnings.extend(result.warnings);
        }
        Err(e) => errors.push(e),
    }

    if errors.is_empty() {
        Ok(ProjectLoadResult { index, warnings })
    } else {
        Err(errors)
    }
}

/// Find RFC JSON by ID
pub fn find_rfc_json(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let rfc_dir = config.rfc_dir().join(rfc_id);
    find_rfc_in_dir(&rfc_dir)
}

pub fn find_rfc_toml(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let path = config.rfc_dir().join(rfc_id).join("rfc.toml");
    path.exists().then_some(path)
}

/// Find clause JSON by full ID (e.g., RFC-0001:C-PHASE-ORDER)
pub fn find_clause_json(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];

    let clause_path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.toml"));

    if clause_path.exists() {
        Some(clause_path)
    } else {
        let legacy_clause_path = config
            .rfc_dir()
            .join(rfc_id)
            .join("clauses")
            .join(format!("{clause_name}.json"));
        legacy_clause_path.exists().then_some(legacy_clause_path)
    }
}

pub fn find_clause_toml(config: &Config, clause_id: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];
    let clause_path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.toml"));
    clause_path.exists().then_some(clause_path)
}
