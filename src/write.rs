//! JSON and frontmatter mutation utilities.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChangelogEntry, ClauseSpec, RfcSpec};
use anyhow::{Context, Result};
use chrono::Local;
use semver::Version;
use std::path::Path;

/// Version bump level
#[derive(Debug, Clone, Copy)]
pub enum BumpLevel {
    Patch,
    Minor,
    Major,
}

/// Read RFC JSON from file
pub fn read_rfc(path: &Path) -> Result<RfcSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read RFC: {}", path.display()))?;
    let rfc: RfcSpec = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse RFC JSON: {}", path.display()))?;
    Ok(rfc)
}

/// Write RFC JSON to file
pub fn write_rfc(path: &Path, rfc: &RfcSpec) -> Result<()> {
    let content = serde_json::to_string_pretty(rfc)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Read clause JSON from file
pub fn read_clause(path: &Path) -> Result<ClauseSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read clause: {}", path.display()))?;
    let clause: ClauseSpec = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse clause JSON: {}", path.display()))?;
    Ok(clause)
}

/// Write clause JSON to file
pub fn write_clause(path: &Path, clause: &ClauseSpec) -> Result<()> {
    let content = serde_json::to_string_pretty(clause)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Bump RFC version and add changelog entry
pub fn bump_rfc_version(rfc: &mut RfcSpec, level: BumpLevel, summary: &str) -> Result<String> {
    let mut version = Version::parse(&rfc.version)
        .with_context(|| format!("Invalid version: {}", rfc.version))?;

    match level {
        BumpLevel::Patch => version.patch += 1,
        BumpLevel::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        BumpLevel::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
    }

    let new_version = version.to_string();
    rfc.version = new_version.clone();
    rfc.updated = Some(today());

    // Add changelog entry
    rfc.changelog.insert(
        0,
        ChangelogEntry {
            version: new_version.clone(),
            date: today(),
            summary: summary.to_string(),
            changes: vec![],
        },
    );

    Ok(new_version)
}

/// Add a change to the current version's changelog
pub fn add_changelog_change(rfc: &mut RfcSpec, change: &str) -> Result<()> {
    if let Some(entry) = rfc.changelog.first_mut() {
        entry.changes.push(change.to_string());
    } else {
        anyhow::bail!("No changelog entry exists. Bump version first.");
    }
    Ok(())
}

/// Get today's date in ISO format
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Update a field in RFC JSON
pub fn update_rfc_field(rfc: &mut RfcSpec, field: &str, value: &str) -> Result<(), Diagnostic> {
    match field {
        "status" => {
            rfc.status = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0101RfcSchemaInvalid,
                    format!("Invalid status value: {value}"),
                    "",
                )
            })?;
        }
        "phase" => {
            rfc.phase = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0101RfcSchemaInvalid,
                    format!("Invalid phase value: {value}"),
                    "",
                )
            })?;
        }
        "title" => rfc.title = value.to_string(),
        "version" => rfc.version = value.to_string(),
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0101RfcSchemaInvalid,
                format!("Unknown field: {field}"),
                "",
            ));
        }
    }
    rfc.updated = Some(today());
    Ok(())
}

/// Update a field in clause JSON
pub fn update_clause_field(
    clause: &mut ClauseSpec,
    field: &str,
    value: &str,
) -> Result<(), Diagnostic> {
    match field {
        "text" => clause.text = value.to_string(),
        "title" => clause.title = value.to_string(),
        "status" => {
            clause.status = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!("Invalid status value: {value}"),
                    "",
                )
            })?;
        }
        "kind" => {
            clause.kind = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!("Invalid kind value: {value}"),
                    "",
                )
            })?;
        }
        "superseded_by" => {
            clause.superseded_by = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
        "since" => clause.since = Some(value.to_string()),
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0201ClauseSchemaInvalid,
                format!("Unknown field: {field}"),
                "",
            ));
        }
    }
    Ok(())
}
