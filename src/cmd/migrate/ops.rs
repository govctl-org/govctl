use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
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

pub(super) fn execute_ops(config: &Config, ops: &[FileOp]) -> anyhow::Result<()> {
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
        )
        .into());
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

fn materialize_stage(stage_root: &Path, ops: &[FileOp]) -> anyhow::Result<()> {
    for (i, op) in ops.iter().enumerate() {
        if let FileOp::Write { content, .. } = op {
            let staged = stage_root.join(format!("{i}"));
            fs::write(&staged, content)
                .map_err(|err| io_error(&staged, "write migration staged file", err))?;
        }
    }
    Ok(())
}

fn commit_ops(stage_root: &Path, backup_root: &Path, ops: &[FileOp]) -> anyhow::Result<()> {
    let mut applied: Vec<usize> = Vec::new();

    let result = (|| -> anyhow::Result<()> {
        for (i, op) in ops.iter().enumerate() {
            let backup_path = backup_root.join(format!("{i}"));
            match op {
                FileOp::Write { path, .. } => {
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
                }
                FileOp::Delete { path } => {
                    if path.exists() {
                        fs::copy(path, &backup_path)
                            .map_err(|err| io_error(path, "backup file before deletion", err))?;
                        fs::remove_file(path)
                            .map_err(|err| io_error(path, "delete migrated legacy file", err))?;
                    }
                }
            }
            applied.push(i);
        }
        Ok(())
    })();

    if result.is_err() {
        for &i in applied.iter().rev() {
            let backup_path = backup_root.join(format!("{i}"));
            if !backup_path.exists() {
                continue;
            }
            match &ops[i] {
                FileOp::Write { path, .. } | FileOp::Delete { path } => {
                    let _ = fs::copy(&backup_path, path);
                }
            }
        }
    }

    result
}

fn io_error(path: &Path, action: &str, err: io::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0901IoError,
        format!("Failed to {action}: {err}"),
        path.display().to_string(),
    )
}
