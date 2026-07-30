use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A single file operation produced by a migration step.
#[derive(Debug, Clone)]
pub(super) enum FileOp {
    Write {
        path: PathBuf,
        content: String,
    },
    #[allow(dead_code)]
    Delete {
        path: PathBuf,
    },
}

pub(super) fn preview_ops(config: &Config, ops: &[FileOp]) {
    for op in ops {
        match op {
            FileOp::Write { path, content } => {
                ui::dry_run_file_preview(&config.display_path(path), content);
            }
            FileOp::Delete { path } => {
                ui::info(format!(
                    "[DRY RUN] Would delete: {}",
                    config.display_path(path).display()
                ));
            }
        }
    }
}

pub(super) fn execute_ops(config: &Config, ops: &[FileOp]) -> DiagnosticResult<()> {
    let gov_root = &config.gov_root;
    let stage_root = gov_root.join(".migrate-stage");
    let backup_root = gov_root.join(".migrate-backup");

    if stage_root.exists() || backup_root.exists() {
        let mut conflicts = Vec::new();
        if stage_root.exists() {
            conflicts.push(config.display_path(&stage_root).display().to_string());
        }
        if backup_root.exists() {
            conflicts.push(config.display_path(&backup_root).display().to_string());
        }
        return Err(Diagnostic::new(
            DiagnosticCode::E0504PathConflict,
            format!(
                "Migration staging directories already exist: {}",
                conflicts.join(", ")
            ),
            config.display_path(gov_root).display().to_string(),
        ));
    }

    fs::create_dir_all(&stage_root)
        .map_err(|err| io_error(&stage_root, "create migration stage directory", err))?;
    if let Err(err) = fs::create_dir_all(&backup_root) {
        let operation_error = io_error(&backup_root, "create migration backup directory", err);
        cleanup_dir(&stage_root);
        return Err(operation_error);
    }

    // Stage: write all new content to staging area
    if let Err(err) = materialize_stage(&stage_root, ops) {
        cleanup_transaction_state(&stage_root, &backup_root);
        return Err(err);
    }

    // Commit: backup originals then apply staged content
    let result = commit_ops(&stage_root, &backup_root, ops);
    match result {
        Ok(_) => {
            cleanup_transaction_state(&stage_root, &backup_root);
            Ok(())
        }
        Err(CommitFailure::RolledBack(err)) => {
            cleanup_transaction_state(&stage_root, &backup_root);
            Err(err)
        }
        Err(CommitFailure::RollbackFailed {
            operation_error,
            rollback_error,
        }) => Err(retain_recovery_error(
            config,
            operation_error,
            rollback_error,
            &stage_root,
            &backup_root,
        )),
    }
}

fn materialize_stage(stage_root: &Path, ops: &[FileOp]) -> DiagnosticResult<()> {
    for (i, op) in ops.iter().enumerate() {
        if let FileOp::Write { content, .. } = op {
            let staged = stage_root.join(format!("{i}"));
            fs::write(&staged, content)
                .map_err(|err| io_error(&staged, "write migration staged file", err))?;
        }
    }
    Ok(())
}

fn commit_ops(
    stage_root: &Path,
    backup_root: &Path,
    ops: &[FileOp],
) -> Result<Vec<AppliedOp>, CommitFailure> {
    let mut applied: Vec<AppliedOp> = Vec::new();

    let result = (|| -> DiagnosticResult<()> {
        for (i, op) in ops.iter().enumerate() {
            let backup_path = backup_root.join(format!("{i}"));
            match op {
                FileOp::Write { path, .. } => {
                    let existed =
                        backup_existing_file(path, &backup_path, "backup file before migration")?;
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).map_err(|err| {
                            io_error(parent, "create migration target directory", err)
                        })?;
                    }
                    let staged = stage_root.join(format!("{i}"));
                    if existed {
                        applied.push(AppliedOp::Restore {
                            path: path.clone(),
                            backup_path,
                        });
                    } else {
                        applied.push(AppliedOp::RemoveCreated { path: path.clone() });
                    }
                    fs::copy(&staged, path)
                        .map_err(|err| io_error(path, "apply migrated file", err))?;
                }
                FileOp::Delete { path } => {
                    if backup_existing_file(path, &backup_path, "backup file before deletion")? {
                        applied.push(AppliedOp::Restore {
                            path: path.clone(),
                            backup_path,
                        });
                        fs::remove_file(path)
                            .map_err(|err| io_error(path, "delete migrated legacy file", err))?;
                    }
                }
            }
        }
        Ok(())
    })();

    match result {
        Ok(()) => Ok(applied),
        Err(operation_error) => match rollback_applied(&applied) {
            Ok(()) => Err(CommitFailure::RolledBack(operation_error)),
            Err(rollback_error) => Err(CommitFailure::RollbackFailed {
                operation_error,
                rollback_error,
            }),
        },
    }
}

fn retain_recovery_error(
    config: &Config,
    operation_error: Diagnostic,
    rollback_error: Diagnostic,
    stage_root: &Path,
    backup_root: &Path,
) -> Diagnostic {
    cleanup_dir(stage_root);
    let backup_display = config.display_path(backup_root).display().to_string();
    Diagnostic::new(
        DiagnosticCode::E0903UnexpectedError,
        format!(
            "{}; transaction rollback failed; repository restoration may be incomplete: {}; recovery backup retained at {}",
            operation_error.message, rollback_error.message, backup_display
        ),
        backup_display,
    )
}

fn rollback_applied(applied: &[AppliedOp]) -> DiagnosticResult<()> {
    let mut first_error = None;
    for op in applied.iter().rev() {
        let result = match op {
            AppliedOp::Restore { path, backup_path } => fs::copy(backup_path, path)
                .map(|_| ())
                .map_err(|err| io_error(path, "restore file during migration rollback", err)),
            AppliedOp::RemoveCreated { path } => match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(io_error(
                    path,
                    "remove created file during migration rollback",
                    err,
                )),
            },
        };
        if first_error.is_none() {
            first_error = result.err();
        }
    }

    first_error.map_or(Ok(()), Err)
}

fn cleanup_transaction_state(stage_root: &Path, backup_root: &Path) {
    cleanup_dir(stage_root);
    cleanup_dir(backup_root);
}

fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn backup_existing_file(path: &Path, backup_path: &Path, action: &str) -> DiagnosticResult<bool> {
    if !path.exists() {
        return Ok(false);
    }
    fs::copy(path, backup_path).map_err(|err| io_error(path, action, err))?;
    Ok(true)
}

enum AppliedOp {
    Restore { path: PathBuf, backup_path: PathBuf },
    RemoveCreated { path: PathBuf },
}

enum CommitFailure {
    RolledBack(Diagnostic),
    RollbackFailed {
        operation_error: Diagnostic,
        rollback_error: Diagnostic,
    },
}

fn io_error(path: &Path, action: &str, err: io::Error) -> Diagnostic {
    Diagnostic::io_error(action, err, path.display().to_string())
}

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
