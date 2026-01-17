//! JSON and frontmatter mutation utilities.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChangelogEntry, ClauseSpec, RfcSpec};
use crate::ui;
use anyhow::{Context, Result};
use chrono::Local;
use semver::Version;
use std::path::Path;

/// Write operation mode (per ADR-0006).
///
/// Controls whether write operations execute or just preview.
#[derive(Debug, Clone, Copy, Default)]
pub enum WriteOp {
    /// Actually write to disk
    #[default]
    Execute,
    /// Preview only: show what would be written
    Preview,
}

impl WriteOp {
    /// Create WriteOp from dry_run boolean flag
    pub fn from_dry_run(dry_run: bool) -> Self {
        if dry_run {
            WriteOp::Preview
        } else {
            WriteOp::Execute
        }
    }

    /// Returns true if this is a preview/dry-run operation
    pub fn is_preview(&self) -> bool {
        matches!(self, WriteOp::Preview)
    }
}

/// Write content to a file, respecting WriteOp mode.
///
/// In Preview mode, shows what would be written instead of writing.
pub fn write_file(path: &Path, content: &str, op: WriteOp) -> Result<()> {
    match op {
        WriteOp::Execute => {
            std::fs::write(path, content)?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(path, content);
        }
    }
    Ok(())
}

/// Create a directory, respecting WriteOp mode.
///
/// In Preview mode, shows what directory would be created.
pub fn create_dir_all(path: &Path, op: WriteOp) -> Result<()> {
    match op {
        WriteOp::Execute => {
            std::fs::create_dir_all(path)?;
        }
        WriteOp::Preview => {
            ui::dry_run_mkdir(path);
        }
    }
    Ok(())
}

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
pub fn write_rfc(path: &Path, rfc: &RfcSpec, op: WriteOp) -> Result<()> {
    let content = serde_json::to_string_pretty(rfc)?;
    write_file(path, &content, op)
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
pub fn write_clause(path: &Path, clause: &ClauseSpec, op: WriteOp) -> Result<()> {
    let content = serde_json::to_string_pretty(clause)?;
    write_file(path, &content, op)
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

    // Add changelog entry (Keep a Changelog format)
    rfc.changelog.insert(
        0,
        ChangelogEntry {
            version: new_version.clone(),
            date: today(),
            notes: Some(summary.to_string()),
            added: vec![],
            changed: vec![],
            deprecated: vec![],
            removed: vec![],
            fixed: vec![],
            security: vec![],
        },
    );

    Ok(new_version)
}

/// Add a change to the current version's changelog (defaults to 'added' category)
pub fn add_changelog_change(rfc: &mut RfcSpec, change: &str) -> Result<()> {
    if let Some(entry) = rfc.changelog.first_mut() {
        entry.added.push(change.to_string());
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
