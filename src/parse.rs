//! TOML parsing for ADR and Work Item files.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, AdrSpec, WorkItemEntry, WorkItemSpec};
use std::path::Path;

/// Load all ADRs from the adr directory
pub fn load_adrs(config: &Config) -> Result<Vec<AdrEntry>, Diagnostic> {
    let adr_dir = config.adr_dir();
    if !adr_dir.exists() {
        return Ok(vec![]);
    }

    let mut adrs = Vec::new();
    let entries = std::fs::read_dir(&adr_dir).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            adr_dir.display().to_string(),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load_adr(&path) {
                Ok(adr) => adrs.push(adr),
                Err(_) => continue, // Skip invalid files
            }
        }
    }

    Ok(adrs)
}

/// Load a single ADR from TOML file
pub fn load_adr(path: &Path) -> Result<AdrEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let spec: AdrSpec = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0301AdrSchemaInvalid,
            format!("Invalid TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    Ok(AdrEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Write an ADR to TOML file
pub fn write_adr(path: &Path, spec: &AdrSpec) -> Result<(), Diagnostic> {
    let content = toml::to_string_pretty(spec).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    std::fs::write(path, content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    Ok(())
}

/// Load all work items from the work directory
pub fn load_work_items(config: &Config) -> Result<Vec<WorkItemEntry>, Diagnostic> {
    let work_dir = config.work_dir();
    if !work_dir.exists() {
        return Ok(vec![]);
    }

    let mut items = Vec::new();
    let entries = std::fs::read_dir(&work_dir).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            work_dir.display().to_string(),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load_work_item(&path) {
                Ok(item) => items.push(item),
                Err(_) => continue, // Skip invalid files
            }
        }
    }

    Ok(items)
}

/// Load a single work item from TOML file
pub fn load_work_item(path: &Path) -> Result<WorkItemEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let spec: WorkItemSpec = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0401WorkSchemaInvalid,
            format!("Invalid TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    Ok(WorkItemEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Write a work item to TOML file
pub fn write_work_item(path: &Path, spec: &WorkItemSpec) -> Result<(), Diagnostic> {
    let content = toml::to_string_pretty(spec).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    std::fs::write(path, content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    Ok(())
}
