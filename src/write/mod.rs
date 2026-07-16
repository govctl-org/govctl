//! Artifact write utilities.
//!
//! Implements [[ADR-0006]] global dry-run support for content-modifying commands.
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

mod artifact;
mod artifact_io;
mod artifact_normalize;
mod changelog;

pub use artifact::{read_clause, read_rfc, write_clause, write_rfc};
pub use artifact_normalize::{normalize_clause_value, normalize_rfc_value};
pub use changelog::{
    BumpLevel, ParsedChange, add_changelog_change, bump_rfc_version, current_changelog_entry,
    current_changelog_entry_mut, today,
};

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
            atomic_write_file(path, content, output_path)?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(output_path, content);
        }
    }
    Ok(())
}

// Write and sync in the target directory before replacing the destination so
// returned lifecycle errors can restore prior content per
// [[RFC-0002:C-LIFECYCLE-VERBS]].
fn atomic_write_file(path: &Path, content: &str, output_path: &Path) -> DiagnosticResult<()> {
    let (target_path, existing_permissions) = inspect_write_target(path, output_path)?;
    let parent = target_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let mut builder = tempfile::Builder::new();
    builder.prefix(".govctl-write-").suffix(".tmp");
    if let Some(ref permissions) = existing_permissions {
        builder.permissions(permissions.clone());
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o666));
        }
    }

    let mut temporary = builder.tempfile_in(parent).map_err(|err| {
        Diagnostic::io_error(
            "create temporary file",
            err,
            output_path.display().to_string(),
        )
    })?;
    temporary.write_all(content.as_bytes()).map_err(|err| {
        Diagnostic::io_error(
            "write temporary file",
            err,
            output_path.display().to_string(),
        )
    })?;
    if let Some(permissions) = existing_permissions {
        temporary
            .as_file()
            .set_permissions(permissions)
            .map_err(|err| {
                Diagnostic::io_error(
                    "set temporary file permissions",
                    err,
                    output_path.display().to_string(),
                )
            })?;
    }
    temporary.as_file().sync_all().map_err(|err| {
        Diagnostic::io_error(
            "sync temporary file",
            err,
            output_path.display().to_string(),
        )
    })?;
    temporary.persist(&target_path).map_err(|err| {
        Diagnostic::io_error("replace file", err.error, output_path.display().to_string())
    })?;
    Ok(())
}

fn inspect_write_target(
    path: &Path,
    output_path: &Path,
) -> DiagnosticResult<(PathBuf, Option<std::fs::Permissions>)> {
    let target_path = resolve_write_target(path, output_path)?;
    match std::fs::metadata(&target_path) {
        Ok(metadata) => {
            OpenOptions::new()
                .write(true)
                .open(&target_path)
                .map_err(|err| {
                    Diagnostic::io_error(
                        "open file for writing",
                        err,
                        output_path.display().to_string(),
                    )
                })?;
            Ok((target_path, Some(metadata.permissions())))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((target_path, None)),
        Err(err) => Err(Diagnostic::io_error(
            "read file metadata",
            err,
            output_path.display().to_string(),
        )),
    }
}

fn resolve_write_target(path: &Path, output_path: &Path) -> DiagnosticResult<PathBuf> {
    const MAX_SYMLINK_DEPTH: usize = 40;

    let mut current = path.to_path_buf();
    let mut visited = HashSet::new();
    let mut traversed = 0;
    loop {
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if traversed == MAX_SYMLINK_DEPTH || !visited.insert(current.clone()) {
                    break;
                }
                traversed += 1;
                let link = std::fs::read_link(&current).map_err(|err| {
                    Diagnostic::io_error(
                        "resolve symbolic link",
                        err,
                        output_path.display().to_string(),
                    )
                })?;
                current = if link.is_absolute() {
                    link
                } else {
                    current
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(link)
                };
            }
            Ok(_) => return Ok(current),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(current),
            Err(err) => {
                return Err(Diagnostic::io_error(
                    "read file metadata",
                    err,
                    output_path.display().to_string(),
                ));
            }
        }
    }

    Err(Diagnostic::io_error(
        "resolve symbolic link",
        std::io::Error::other("too many symbolic links"),
        output_path.display().to_string(),
    ))
}

struct FileSnapshot {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

/// Run a group of artifact writes with preflight checks and rollback on error.
pub fn with_file_transaction<T>(
    paths: &[&Path],
    op: WriteOp,
    operation: impl FnOnce() -> DiagnosticResult<T>,
) -> DiagnosticResult<T> {
    if op.is_preview() {
        return operation();
    }

    let mut targets = HashSet::new();
    let mut snapshots = Vec::new();
    for path in paths {
        let (target, _) = inspect_write_target(path, path)?;
        if !targets.insert(target.clone()) {
            continue;
        }
        let content = match std::fs::read(&target) {
            Ok(content) => Some(content),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                return Err(Diagnostic::io_error(
                    "read file before transaction",
                    err,
                    path.display().to_string(),
                ));
            }
        };
        snapshots.push(FileSnapshot {
            path: target,
            content,
        });
    }

    match operation() {
        Ok(value) => Ok(value),
        Err(operation_error) => match rollback_files(snapshots) {
            Ok(()) => Err(operation_error),
            Err(rollback_error) => Err(Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                format!(
                    "{}; transaction rollback failed; governed-artifact restoration may be incomplete: {}",
                    operation_error.message, rollback_error.message
                ),
                operation_error.file,
            )),
        },
    }
}

fn rollback_files(snapshots: Vec<FileSnapshot>) -> DiagnosticResult<()> {
    for snapshot in snapshots.into_iter().rev() {
        match snapshot.content {
            Some(content) => {
                let current = std::fs::read(&snapshot.path).ok();
                if current.as_deref() == Some(content.as_slice()) {
                    continue;
                }
                let content = std::str::from_utf8(&content).map_err(|err| {
                    Diagnostic::new(
                        DiagnosticCode::E0903UnexpectedError,
                        format!("Failed to restore non-UTF-8 artifact: {err}"),
                        snapshot.path.display().to_string(),
                    )
                })?;
                atomic_write_file(&snapshot.path, content, &snapshot.path)?;
            }
            None => match std::fs::remove_file(&snapshot.path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(Diagnostic::io_error(
                        "remove file during transaction rollback",
                        err,
                        snapshot.path.display().to_string(),
                    ));
                }
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    #[test]
    fn failed_write_preserves_existing_content() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("artifact.toml");
        std::fs::write(&path, "original")?;

        let original_dir_permissions = std::fs::metadata(dir.path())?.permissions();
        let mut unwritable_dir_permissions = original_dir_permissions.clone();
        unwritable_dir_permissions.set_mode(original_dir_permissions.mode() & !0o222);
        std::fs::set_permissions(dir.path(), unwritable_dir_permissions)?;

        let result = write_file(&path, "replacement", WriteOp::Execute, None);
        std::fs::set_permissions(dir.path(), original_dir_permissions)?;

        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(path)?, "original");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn successful_write_preserves_existing_permissions() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("artifact.toml");
        std::fs::write(&path, "original")?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))?;

        write_file(&path, "replacement", WriteOp::Execute, None)?;

        assert_eq!(std::fs::read_to_string(&path)?, "replacement");
        assert_eq!(std::fs::metadata(path)?.permissions().mode() & 0o777, 0o640);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn write_rejects_read_only_existing_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("artifact.toml");
        std::fs::write(&path, "original")?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444))?;

        let result = write_file(&path, "replacement", WriteOp::Execute, None);

        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(path)?, "original");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn successful_write_preserves_symlink() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("target.toml");
        let link = dir.path().join("artifact.toml");
        std::fs::write(&target, "original")?;
        std::os::unix::fs::symlink(&target, &link)?;

        write_file(&link, "replacement", WriteOp::Execute, None)?;

        assert!(std::fs::symlink_metadata(&link)?.file_type().is_symlink());
        assert_eq!(std::fs::read_to_string(target)?, "replacement");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn successful_write_follows_dangling_symlink() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("target.toml");
        let link = dir.path().join("artifact.toml");
        std::os::unix::fs::symlink("target.toml", &link)?;

        write_file(&link, "replacement", WriteOp::Execute, None)?;

        assert!(std::fs::symlink_metadata(&link)?.file_type().is_symlink());
        assert_eq!(std::fs::read_to_string(target)?, "replacement");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn successful_write_follows_forty_symlinks() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("target.toml");
        std::fs::write(&target, "original")?;
        for index in (0..40).rev() {
            let destination = if index == 39 {
                PathBuf::from("target.toml")
            } else {
                PathBuf::from(format!("link-{}.toml", index + 1))
            };
            std::os::unix::fs::symlink(destination, dir.path().join(format!("link-{index}.toml")))?;
        }

        write_file(
            &dir.path().join("link-0.toml"),
            "replacement",
            WriteOp::Execute,
            None,
        )?;

        assert_eq!(std::fs::read_to_string(target)?, "replacement");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn write_rejects_forty_one_symlinks() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("target.toml");
        std::fs::write(&target, "original")?;
        for index in (0..41).rev() {
            let destination = if index == 40 {
                PathBuf::from("target.toml")
            } else {
                PathBuf::from(format!("link-{}.toml", index + 1))
            };
            std::os::unix::fs::symlink(destination, dir.path().join(format!("link-{index}.toml")))?;
        }

        let result = write_file(
            &dir.path().join("link-0.toml"),
            "replacement",
            WriteOp::Execute,
            None,
        );

        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(target)?, "original");
        Ok(())
    }

    #[test]
    fn file_transaction_restores_prior_writes_after_failure()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let first = dir.path().join("first.toml");
        let second = dir.path().join("second.toml");
        std::fs::write(&first, "first-original")?;
        std::fs::write(&second, "second-original")?;

        let result = with_file_transaction(
            &[first.as_path(), second.as_path()],
            WriteOp::Execute,
            || {
                write_file(&first, "first-replacement", WriteOp::Execute, None)?;
                Err::<(), _>(Diagnostic::new(
                    crate::diagnostic::DiagnosticCode::E0903UnexpectedError,
                    "injected write failure",
                    second.display().to_string(),
                ))
            },
        );

        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(first)?, "first-original");
        assert_eq!(std::fs::read_to_string(second)?, "second-original");
        Ok(())
    }

    #[test]
    fn file_transaction_restores_deleted_file_after_failure()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("artifact.toml");
        std::fs::write(&path, "original")?;

        let result = with_file_transaction(&[path.as_path()], WriteOp::Execute, || {
            std::fs::remove_file(&path).map_err(|err| {
                Diagnostic::io_error("delete file", err, path.display().to_string())
            })?;
            Err::<(), _>(Diagnostic::new(
                crate::diagnostic::DiagnosticCode::E0903UnexpectedError,
                "injected failure after deletion",
                path.display().to_string(),
            ))
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(path)?, "original");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn file_transaction_reports_rollback_failure() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("artifact.toml");
        std::fs::write(&path, "original")?;

        let result = with_file_transaction(&[path.as_path()], WriteOp::Execute, || {
            write_file(&path, "replacement", WriteOp::Execute, None)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).map_err(
                |err| Diagnostic::io_error("make file read-only", err, path.display().to_string()),
            )?;
            Err::<(), _>(Diagnostic::new(
                crate::diagnostic::DiagnosticCode::E0903UnexpectedError,
                "injected write failure",
                path.display().to_string(),
            ))
        });
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;

        let Err(error) = result else {
            return Err("rollback unexpectedly succeeded for a read-only file".into());
        };
        assert_eq!(
            error.code,
            crate::diagnostic::DiagnosticCode::E0903UnexpectedError
        );
        assert!(error.message.contains("transaction rollback failed"));
        assert!(error.message.contains("restoration may be incomplete"));
        Ok(())
    }

    #[test]
    fn successful_write_replaces_existing_content() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("artifact.toml");
        std::fs::write(&path, "original")?;

        write_file(&path, "replacement", WriteOp::Execute, None)?;

        assert_eq!(std::fs::read_to_string(path)?, "replacement");
        Ok(())
    }
}
