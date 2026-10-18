//! Project support file synchronization shared by init, migrate, and check.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::ui;
use crate::write::{WriteOp, write_file};
use std::io::ErrorKind;
use std::path::PathBuf;

// Implements [[RFC-0002:C-GLOBAL-COMMANDS]]: init/migrate maintain local-state ignore entries.
const LOCAL_STATE_GITIGNORE_ENTRIES: &[&str] = &[".govctl.lock", ".govctl/"];

// Implements [[RFC-0002:C-GLOBAL-COMMANDS]]: migrate refreshes local-state
// .gitignore entries regardless of schema version.
pub(crate) fn ensure_local_state_gitignore_entries(
    config: &Config,
    op: WriteOp,
) -> DiagnosticResult<usize> {
    let gitignore_path = gitignore_path(config);
    let display_path = config.display_path(&gitignore_path);

    match std::fs::read_to_string(&gitignore_path) {
        Ok(content) => {
            let missing_entries = entries_to_sync(&content, &effective_ignore_status(config));
            if missing_entries.is_empty() {
                return Ok(0);
            }

            let missing_content = missing_entries.join("\n");
            let new_content = if content.ends_with('\n') {
                format!("{content}{missing_content}\n")
            } else {
                format!("{content}\n{missing_content}\n")
            };
            write_file(&gitignore_path, &new_content, op, Some(&display_path))?;
            if !op.is_preview() {
                ui::info(format!(
                    "Added local govctl state entries to .gitignore: {}",
                    missing_entries.join(", ")
                ));
            }
            Ok(missing_entries.len())
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let missing_entries = entries_to_sync("", &effective_ignore_status(config));
            if missing_entries.is_empty() {
                return Ok(0);
            }
            let content = format!("# govctl local state\n{}\n", missing_entries.join("\n"));
            write_file(&gitignore_path, &content, op, Some(&display_path))?;
            if !op.is_preview() {
                ui::created_path(&display_path);
            }
            Ok(missing_entries.len())
        }
        Err(err) => Err(Diagnostic::io_error(
            "read .gitignore",
            err,
            display_path.display().to_string(),
        )),
    }
}

pub(crate) fn local_state_gitignore_needs_sync(config: &Config) -> DiagnosticResult<bool> {
    let gitignore_path = gitignore_path(config);
    match std::fs::read_to_string(&gitignore_path) {
        Ok(content) => Ok(!entries_to_sync(&content, &effective_ignore_status(config)).is_empty()),
        Err(err) if err.kind() == ErrorKind::NotFound => {
            Ok(!entries_to_sync("", &effective_ignore_status(config)).is_empty())
        }
        Err(err) => Err(Diagnostic::io_error(
            "read .gitignore",
            err,
            config.display_path(&gitignore_path).display().to_string(),
        )),
    }
}

// Implements [[RFC-0002:C-GLOBAL-COMMANDS]]: check warns when govctl-managed
// local-state paths are not effectively ignored by Git.
pub(crate) fn local_state_gitignore_diagnostics(config: &Config) -> Diagnostics {
    let gitignore_path = gitignore_path(config);
    let display_path = config.display_path(&gitignore_path);
    let states = effective_ignore_status(config);
    if states
        .iter()
        .copied()
        .all(|state| state == IgnoreState::Ignored)
    {
        return vec![];
    }
    let missing_entries = match std::fs::read_to_string(&gitignore_path) {
        Ok(content) => entries_to_sync(&content, &states),
        Err(err) if err.kind() == ErrorKind::NotFound => entries_to_sync("", &states),
        Err(err) => {
            return vec![Diagnostic::io_error(
                "read .gitignore",
                err,
                display_path.display().to_string(),
            )];
        }
    };

    if missing_entries.is_empty() {
        return vec![];
    }

    vec![Diagnostic::new(
        DiagnosticCode::W0111ProjectSupportOutdated,
        format!(
            "Local govctl state paths are not effectively ignored: {}. Run `govctl migrate` to refresh local-state ignore entries.",
            missing_entries.join(", ")
        ),
        display_path.display().to_string(),
    )]
}

fn gitignore_path(config: &Config) -> PathBuf {
    config.project_root().join(".gitignore")
}

/// Effective-ignore verdict for one local-state path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IgnoreState {
    /// `git check-ignore` reports the path ignored.
    Ignored,
    /// `git check-ignore` reports the path not ignored.
    Trackable,
    /// The path is already tracked by Git.
    Tracked,
    /// Git could not answer (not a repository, git missing, unexpected
    /// status).
    Indeterminate,
}

/// Local-state entries that still need syncing.
fn entries_to_sync(content: &str, states: &[IgnoreState]) -> Vec<&'static str> {
    LOCAL_STATE_GITIGNORE_ENTRIES
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, entry)| {
            let state = states
                .get(index)
                .copied()
                .unwrap_or(IgnoreState::Indeterminate);
            // A `Trackable` entry is synced even if its literal line already exists: a later
            // negation revoked the earlier rule, and appending the managed entry restores the
            // ignore via last-match-wins.
            let needs_sync = state == IgnoreState::Trackable
                || (state == IgnoreState::Indeterminate
                    && !content.lines().any(|line| line.trim() == entry));
            needs_sync.then_some(entry)
        })
        .collect()
}

/// Per-entry effective-ignore verdict from `git check-ignore`, in
/// `LOCAL_STATE_GITIGNORE_ENTRIES` order.
fn effective_ignore_status(config: &Config) -> Vec<IgnoreState> {
    let root = config.project_root();
    LOCAL_STATE_GITIGNORE_ENTRIES
        .iter()
        .map(|entry| git_ignore_state(root, entry))
        .collect()
}

/// Effective-ignore verdict for `entry` under `root`.
fn git_ignore_state(root: &std::path::Path, entry: &str) -> IgnoreState {
    let plain = root.join(entry).display().to_string();
    // Ignore rules never apply to tracked paths, and `check-ignore` reports
    // them not-ignored: syncing rules for one would pile up futile
    // duplicates that no `migrate` run can satisfy, so skip it without
    // warning. Only an exact index path counts; a tracked child of `.govctl/`
    // must not hide a missing directory ignore rule.
    if git_is_tracked(root, entry) {
        return IgnoreState::Tracked;
    }
    // The directory entry needs its trailing slash: git only applies `dir/`
    // patterns to slash-terminated paths that may not exist yet. `Path`
    // normalization strips it, so restore it from the entry spelling.
    let mut candidate = plain;
    if entry.ends_with('/') && !candidate.ends_with('/') {
        candidate.push('/');
    }
    let output = std::process::Command::new("git")
        .args(["check-ignore", "-q", "--"])
        .arg(candidate)
        .current_dir(root)
        .output();
    match output.ok().and_then(|output| output.status.code()) {
        Some(0) => IgnoreState::Ignored,
        Some(1) => IgnoreState::Trackable,
        _ => IgnoreState::Indeterminate,
    }
}

/// Whether the exact path `entry` is already tracked by Git.
///
/// Directory spellings such as `.govctl/` are not index entries. A tracked
/// child must not make the directory look tracked, or an unignored directory
/// would skip the ignore check.
fn git_is_tracked(root: &std::path::Path, entry: &str) -> bool {
    if entry.ends_with('/') {
        return false;
    }
    std::process::Command::new("git")
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(entry)
        .current_dir(root)
        .output()
        .ok()
        .and_then(|output| output.status.code())
        == Some(0)
}
