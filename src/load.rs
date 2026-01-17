//! JSON loading for RFC and clause files.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseEntry, ClauseSpec, ProjectIndex, RfcIndex, RfcSpec};
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

/// Load all RFCs from the spec directory
pub fn load_rfcs(config: &Config) -> Result<Vec<RfcIndex>, LoadError> {
    let rfcs_dir = config.rfcs_dir();
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
        if path.is_dir() {
            let rfc_json = path.join("rfc.json");
            if rfc_json.exists() {
                let rfc_index = load_rfc(&rfc_json)?;
                rfcs.push(rfc_index);
            }
        }
    }

    Ok(rfcs)
}

/// Load a single RFC and its clauses
pub fn load_rfc(rfc_json: &Path) -> Result<RfcIndex, LoadError> {
    let content = std::fs::read_to_string(rfc_json).map_err(|e| LoadError::Io {
        file: rfc_json.display().to_string(),
        message: e.to_string(),
    })?;

    let rfc: RfcSpec = serde_json::from_str(&content).map_err(|e| LoadError::Json {
        file: rfc_json.display().to_string(),
        message: e.to_string(),
    })?;

    let rfc_dir = rfc_json.parent().unwrap();
    let mut clauses = Vec::new();

    // Load all clauses referenced in sections
    for section in &rfc.sections {
        for clause_path in &section.clauses {
            // Validate path doesn't escape
            if clause_path.contains("..") {
                return Err(LoadError::ClausePathInvalid {
                    file: rfc_json.display().to_string(),
                    clause: clause_path.clone(),
                });
            }

            let full_path = rfc_dir.join(clause_path);
            if full_path.exists() {
                let clause = load_clause(&full_path)?;
                clauses.push(clause);
            }
        }
    }

    Ok(RfcIndex {
        rfc,
        clauses,
        path: rfc_json.to_path_buf(),
    })
}

/// Load a single clause
pub fn load_clause(path: &Path) -> Result<ClauseEntry, LoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        file: path.display().to_string(),
        message: e.to_string(),
    })?;

    let spec: ClauseSpec = serde_json::from_str(&content).map_err(|e| LoadError::Json {
        file: path.display().to_string(),
        message: e.to_string(),
    })?;

    Ok(ClauseEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Load full project index (RFCs, ADRs, Work Items)
pub fn load_project(config: &Config) -> Result<ProjectIndex, Vec<Diagnostic>> {
    let mut index = ProjectIndex::default();
    let mut errors = Vec::new();

    // Load RFCs
    match load_rfcs(config) {
        Ok(rfcs) => index.rfcs = rfcs,
        Err(e) => errors.push(e.into()),
    }

    // Load ADRs
    match crate::parse::load_adrs(config) {
        Ok(adrs) => index.adrs = adrs,
        Err(e) => errors.push(e),
    }

    // Load Work Items
    match crate::parse::load_work_items(config) {
        Ok(items) => index.work_items = items,
        Err(e) => errors.push(e),
    }

    if errors.is_empty() {
        Ok(index)
    } else {
        Err(errors)
    }
}

/// Find RFC JSON by ID
pub fn find_rfc_json(config: &Config, rfc_id: &str) -> Option<PathBuf> {
    let rfc_dir = config.rfcs_dir().join(rfc_id);
    let rfc_json = rfc_dir.join("rfc.json");
    if rfc_json.exists() {
        Some(rfc_json)
    } else {
        None
    }
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
        .rfcs_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.json"));

    if clause_path.exists() {
        Some(clause_path)
    } else {
        None
    }
}
