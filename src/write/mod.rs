//! JSON and frontmatter mutation utilities.
//!
//! Implements [[ADR-0006]] global dry-run support for content-modifying commands.
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::diagnostic::{Diagnostic, DiagnosticResult};
use crate::ui;
use std::path::Path;

mod artifact;
mod artifact_io;
mod artifact_normalize;
mod changelog;

pub use artifact::{read_clause, read_rfc, write_clause, write_rfc};
pub(crate) use artifact_normalize::{normalize_clause_json, normalize_rfc_json};
pub use artifact_normalize::{normalize_clause_value, normalize_rfc_value};
pub use changelog::{BumpLevel, ParsedChange, add_changelog_change, bump_rfc_version, today};

pub fn parse_changelog_change(change: &str) -> DiagnosticResult<ParsedChange> {
    changelog::parse_changelog_change(change)
}

/// Write operation mode.
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
/// If `display_path` is provided, it's used for the preview output instead of `path`.
pub fn write_file(
    path: &Path,
    content: &str,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::write(path, content).map_err(|err| {
                Diagnostic::io_error("write file", err, output_path.display().to_string())
            })?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(output_path, content);
        }
    }
    Ok(())
}

/// Create a directory, respecting WriteOp mode.
///
/// In Preview mode, shows what directory would be created.
/// If `display_path` is provided, it's used for the preview output instead of `path`.
pub fn create_dir_all(
    path: &Path,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::create_dir_all(path).map_err(|err| {
                Diagnostic::io_error("create directory", err, output_path.display().to_string())
            })?;
        }
        WriteOp::Preview => {
            ui::dry_run_mkdir(output_path);
        }
    }
    Ok(())
}

/// Delete a file, respecting WriteOp mode.
///
/// In Preview mode, shows what would be deleted instead of deleting.
/// If `display_path` is provided, it's used for error messages and preview output.
pub fn delete_file(path: &Path, op: WriteOp, display_path: Option<&Path>) -> DiagnosticResult<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::remove_file(path).map_err(|err| {
                Diagnostic::io_error("delete file", err, output_path.display().to_string())
            })?;
        }
        WriteOp::Preview => {
            ui::info(format!("[DRY RUN] Would delete: {}", output_path.display()));
        }
    }
    Ok(())
}
