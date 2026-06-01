use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A single file operation produced by a migration step.
#[derive(Debug, Clone)]
pub(super) enum FileOp {
    Write { path: PathBuf, content: String },
    Delete { path: PathBuf },
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
    fs::create_dir_all(&backup_root)
        .map_err(|err| io_error(&backup_root, "create migration backup directory", err))?;

    // Stage: write all new content to staging area
    if let Err(err) = materialize_stage(&stage_root, ops) {
        let _ = fs::remove_dir_all(&stage_root);
        let _ = fs::remove_dir_all(&backup_root);
        return Err(err);
    }

    // Commit: backup originals then apply staged content
    let result = commit_ops(&stage_root, &backup_root, ops);
    let _ = fs::remove_dir_all(&stage_root);
    if result.is_ok() {
        let _ = fs::remove_dir_all(&backup_root);
    }
    result
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

fn commit_ops(stage_root: &Path, backup_root: &Path, ops: &[FileOp]) -> DiagnosticResult<()> {
    let mut applied: Vec<AppliedOp> = Vec::new();

    let result = (|| -> DiagnosticResult<()> {
        for (i, op) in ops.iter().enumerate() {
            let backup_path = backup_root.join(format!("{i}"));
            match op {
                FileOp::Write { path, .. } => {
                    let existed = path.exists();
                    if path.exists() {
                        fs::copy(path, &backup_path)
                            .map_err(|err| io_error(path, "backup file before migration", err))?;
                    }
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).map_err(|err| {
                            io_error(parent, "create migration target directory", err)
                        })?;
                    }
                    let staged = stage_root.join(format!("{i}"));
                    fs::copy(&staged, path)
                        .map_err(|err| io_error(path, "apply migrated file", err))?;
                    if existed {
                        applied.push(AppliedOp::Restore {
                            path: path.clone(),
                            backup_path,
                        });
                    } else {
                        applied.push(AppliedOp::RemoveCreated { path: path.clone() });
                    }
                }
                FileOp::Delete { path } => {
                    if path.exists() {
                        fs::copy(path, &backup_path)
                            .map_err(|err| io_error(path, "backup file before deletion", err))?;
                        fs::remove_file(path)
                            .map_err(|err| io_error(path, "delete migrated legacy file", err))?;
                        applied.push(AppliedOp::Restore {
                            path: path.clone(),
                            backup_path,
                        });
                    }
                }
            }
        }
        Ok(())
    })();

    if result.is_err() {
        for op in applied.iter().rev() {
            match op {
                AppliedOp::Restore { path, backup_path } => {
                    let _ = fs::copy(backup_path, path);
                }
                AppliedOp::RemoveCreated { path } => {
                    if path.exists() {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
    }

    result
}

enum AppliedOp {
    Restore { path: PathBuf, backup_path: PathBuf },
    RemoveCreated { path: PathBuf },
}

fn io_error(path: &Path, action: &str, err: io::Error) -> Diagnostic {
    Diagnostic::io_error(action, err, path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config(temp_dir: &tempfile::TempDir) -> Config {
        let mut config = Config {
            gov_root: temp_dir.path().join("gov"),
            ..Config::default()
        };
        config.paths.docs_output = temp_dir.path().join("docs");
        config
    }

    #[test]
    fn execute_ops_removes_created_files_when_later_apply_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let config = test_config(&temp_dir);
        fs::create_dir_all(&config.gov_root)?;
        let created = config.gov_root.join("created.txt");
        let bad_target = config.gov_root.join("bad-target");
        fs::create_dir_all(&bad_target)?;

        let result = execute_ops(
            &config,
            &[
                FileOp::Write {
                    path: created.clone(),
                    content: "created".to_string(),
                },
                FileOp::Write {
                    path: bad_target,
                    content: "cannot replace directory".to_string(),
                },
            ],
        );

        assert!(result.is_err());
        assert!(
            !created.exists(),
            "created migration target should be removed on rollback"
        );
        Ok(())
    }

    #[test]
    fn execute_ops_restores_modified_and_deleted_files_when_later_apply_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let config = test_config(&temp_dir);
        fs::create_dir_all(&config.gov_root)?;
        let modified = config.gov_root.join("modified.txt");
        let deleted = config.gov_root.join("deleted.txt");
        let bad_target = config.gov_root.join("bad-target");
        fs::write(&modified, "old")?;
        fs::write(&deleted, "gone")?;
        fs::create_dir_all(&bad_target)?;

        let result = execute_ops(
            &config,
            &[
                FileOp::Write {
                    path: modified.clone(),
                    content: "new".to_string(),
                },
                FileOp::Delete {
                    path: deleted.clone(),
                },
                FileOp::Write {
                    path: bad_target,
                    content: "cannot replace directory".to_string(),
                },
            ],
        );

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(modified)?, "old");
        assert_eq!(fs::read_to_string(deleted)?, "gone");
        Ok(())
    }
}
