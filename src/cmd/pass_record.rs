//! Checkout pass records.
//!
//! Implements [[RFC-0001:C-PASS-RECORD]] and [[RFC-0002:C-PASS-RECORD-COMMAND]].
//! The stored revision is the commit identifier in shared version-control
//! history, sampled before the record file is created or modified.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::WorkItemEntry;
use crate::ui;
use crate::verification;
use crate::workspace::{self, StorageLookup, VcsKind};

#[derive(Debug)]
pub(crate) struct PassRecord {
    pub(crate) work_id: String,
    pub(crate) revision: String,
    pub(crate) guards: BTreeSet<String>,
}

pub(crate) struct Checkout {
    kind: VcsKind,
    root: PathBuf,
    revision: String,
}

pub(crate) enum InspectError {
    NoVcs,
    Unreadable,
}

/// A supplied record satisfies the guard portion of `done`.
///
/// Does not execute guards and does not modify the record or the work item.
pub(crate) fn require_evidence(
    config: &Config,
    work_item: &WorkItemEntry,
    record_path: &Path,
) -> DiagnosticResult<()> {
    let work_id = work_item.spec.govctl.id.as_str();
    let record = read_record(record_path)?;
    if record.work_id != work_id {
        return Err(reject(
            format!("Pass record work_id does not match {work_id}"),
            work_id,
        ));
    }
    let checkout = match inspect(config.project_root()) {
        Ok(checkout) => checkout,
        Err(InspectError::NoVcs) => {
            return Err(reject(
                "This checkout has no usable commit identifier",
                work_id,
            ));
        }
        Err(InspectError::Unreadable) => {
            ui::diagnostic(&unreadable_warning(work_id));
            return Err(reject(
                "This checkout has no usable commit identifier",
                work_id,
            ));
        }
    };
    let matches = content_matches(&checkout, &record.revision, Some(record_path)).map_err(
        |err| match err {
            InspectError::NoVcs => reject("This checkout has no usable commit identifier", work_id),
            InspectError::Unreadable => {
                ui::diagnostic(&unreadable_warning(work_id));
                reject("This checkout has no usable commit identifier", work_id)
            }
        },
    )?;
    if !matches {
        return Err(reject(
            "Checkout does not match the pass record revision",
            work_id,
        ));
    }
    let effective = verification::effective_required_guard_ids(config, work_item);
    if !guards_satisfy(&effective, &record.guards) {
        return Err(reject(
            "Pass record guards do not match the effective guard set",
            work_id,
        ));
    }
    Ok(())
}

pub(crate) fn unreadable_warning(location: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::W0115RegistryDegraded,
        "Version-control metadata is present but the commit identifier cannot be read",
        location,
    )
}

pub(crate) fn reject(message: impl Into<String>, location: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0820InvalidFieldValue,
        message.into(),
        location,
    )
}

/// The effective set is empty, so a pass record cannot satisfy the gate.
pub(crate) fn empty_guard_set(location: &str) -> Diagnostic {
    reject(
        "The effective guard set is empty, so a pass record cannot satisfy the done gate",
        location,
    )
}

pub(crate) fn guards_satisfy(effective: &[String], recorded: &BTreeSet<String>) -> bool {
    // [[RFC-0001:C-GATE-CONDITIONS]]: a supplied record does not use the empty-set rule.
    if effective.is_empty() {
        return false;
    }
    let effective: BTreeSet<&str> = effective.iter().map(String::as_str).collect();
    recorded.len() == effective.len()
        && recorded
            .iter()
            .all(|guard| effective.contains(guard.as_str()))
}

pub(crate) fn read_record(path: &Path) -> Result<PassRecord, Diagnostic> {
    let location = path.display().to_string();
    let text = std::fs::read_to_string(path)
        .map_err(|err| Diagnostic::io_error("read pass record", err, &location))?;
    let value: toml::Value = toml::from_str(&text)
        .map_err(|err| reject(format!("Pass record is not valid TOML: {err}"), &location))?;
    let Some(table) = value.as_table() else {
        return Err(reject("Pass record must be a TOML table", &location));
    };
    let keys: BTreeSet<&str> = table.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = ["work_id", "revision", "guards"].into_iter().collect();
    if keys != expected {
        return Err(reject(
            "Pass record keys must be exactly work_id, revision, and guards",
            &location,
        ));
    }
    let work_id = required_string(table, "work_id", &location)?;
    let revision = required_string(table, "revision", &location)?;
    let Some(guards_value) = table.get("guards").and_then(toml::Value::as_array) else {
        return Err(reject("Pass record guards must be an array", &location));
    };
    let mut guards = BTreeSet::new();
    for guard in guards_value {
        let Some(guard) = guard.as_str() else {
            return Err(reject(
                "Pass record guards must be an array of strings",
                &location,
            ));
        };
        if guard.is_empty() {
            return Err(reject("Pass record guard IDs must not be empty", &location));
        }
        if !guards.insert(guard.to_string()) {
            return Err(reject(
                format!("Pass record guard '{guard}' is duplicated"),
                &location,
            ));
        }
    }
    Ok(PassRecord {
        work_id,
        revision,
        guards,
    })
}

fn required_string(table: &toml::Table, key: &str, location: &str) -> Result<String, Diagnostic> {
    match table.get(key).and_then(toml::Value::as_str) {
        Some(value) if !value.is_empty() => Ok(value.to_string()),
        _ => Err(reject(
            format!("Pass record {key} must be a non-empty string"),
            location,
        )),
    }
}

#[derive(Serialize)]
struct RecordFile<'a> {
    work_id: &'a str,
    revision: &'a str,
    guards: &'a [String],
}

pub(crate) fn write_record(
    path: &Path,
    work_id: &str,
    revision: &str,
    guards: &[String],
) -> Result<(), Diagnostic> {
    let location = path.display().to_string();
    let body = toml::to_string(&RecordFile {
        work_id,
        revision,
        guards,
    })
    .map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize pass record: {err}"),
            &location,
        )
    })?;
    crate::write::write_file(path, &body, crate::write::WriteOp::Execute, Some(path))
}

pub(crate) fn sample_checkout(dir: &Path) -> Result<Checkout, InspectError> {
    inspect(dir)
}

pub(crate) fn matches_revision(
    checkout: &Checkout,
    revision: &str,
    record_path: Option<&Path>,
) -> Result<bool, InspectError> {
    content_matches(checkout, revision, record_path)
}

pub(crate) fn current_revision(checkout: &Checkout) -> &str {
    &checkout.revision
}

fn inspect(dir: &Path) -> Result<Checkout, InspectError> {
    let (kind, root) = match workspace::shared_storage(dir) {
        StorageLookup::NoVcs => return Err(InspectError::NoVcs),
        StorageLookup::Unavailable => return Err(InspectError::Unreadable),
        StorageLookup::Found(storage) => (storage.kind, storage.repo_root),
    };
    let revision = match kind {
        VcsKind::Git => command_stdout(&root, "git", &["rev-parse", "HEAD"])?,
        VcsKind::Jj => command_stdout(
            &root,
            "jj",
            &[
                "log",
                "--ignore-working-copy",
                "-r",
                "@",
                "--no-graph",
                "-T",
                "commit_id",
            ],
        )?,
    };
    let revision = revision.trim().to_string();
    if revision.is_empty() {
        return Err(InspectError::Unreadable);
    }
    Ok(Checkout {
        kind,
        root,
        revision,
    })
}

fn content_matches(
    checkout: &Checkout,
    revision: &str,
    record_path: Option<&Path>,
) -> Result<bool, InspectError> {
    let differences = match checkout.kind {
        VcsKind::Git => git_differences(&checkout.root, revision)?,
        VcsKind::Jj => jj_differences(&checkout.root, revision)?,
    };
    let allowed = record_path.and_then(|path| record_relative(&checkout.root, path));
    Ok(differences
        .iter()
        .all(|path| allowed.as_deref() == Some(path.as_str())))
}

fn git_differences(root: &Path, revision: &str) -> Result<Vec<String>, InspectError> {
    // `-z` keeps the raw path. Quoted `core.quotepath` text would not match
    // the pass-record file when its name is not ASCII.
    let tracked = command_stdout(
        root,
        "git",
        &[
            "--no-pager",
            "diff",
            "-z",
            "--name-only",
            "--no-renames",
            revision,
        ],
    )?;
    let untracked = command_stdout(
        root,
        "git",
        &["ls-files", "-z", "--others", "--exclude-standard"],
    )?;
    Ok(merge_paths(nul_paths(&tracked), nul_paths(&untracked)))
}

fn jj_differences(root: &Path, revision: &str) -> Result<Vec<String>, InspectError> {
    // `jj diff --name-only` prints each path raw, including non-ASCII names.
    let tracked = command_stdout(root, "jj", &["diff", "--from", revision, "--name-only"])?;
    Ok(line_paths(&tracked))
}

/// Git `-z` paths are already raw and `/`-separated. Trimming or rewriting
/// `\` would make a different filename look like the pass-record file.
fn nul_paths(text: &str) -> Vec<String> {
    text.split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

fn line_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for path in text.split('\n') {
        if path.is_empty() {
            continue;
        }
        if !paths.iter().any(|existing| existing == path) {
            paths.push(path.to_string());
        }
    }
    paths
}

fn merge_paths(mut paths: Vec<String>, more: Vec<String>) -> Vec<String> {
    for path in more {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }
    paths
}

fn record_relative(repo_root: &Path, record_path: &Path) -> Option<String> {
    let absolute = if record_path.is_absolute() {
        record_path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(record_path)
    };
    let repo = std::fs::canonicalize(repo_root).ok()?;
    let anchor = if absolute.exists() {
        std::fs::canonicalize(&absolute).ok()?
    } else {
        let parent = absolute.parent()?;
        let name = absolute.file_name()?;
        std::fs::canonicalize(parent).ok()?.join(name)
    };
    let relative = anchor.strip_prefix(&repo).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(part) => {
                parts.push(part.to_string_lossy().into_owned());
            }
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

fn command_stdout(dir: &Path, program: &str, args: &[&str]) -> Result<String, InspectError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .map_err(|_| InspectError::Unreadable)?;
    if !output.status.success() {
        return Err(InspectError::Unreadable);
    }
    String::from_utf8(output.stdout).map_err(|_| InspectError::Unreadable)
}
