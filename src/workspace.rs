//! Workspace detection for multi-workspace coordination.
//!
//! Implements the primary-workspace determination of
//! [[RFC-0010:C-COMMAND-SCOPE]] and the detection degradation rules of
//! [[RFC-0010:C-REGISTRY]]: no version control is silently single-checkout,
//! while present-but-unreadable metadata or an unresolvable primary name
//! degrades toward caution with a warning.

use crate::config::Config;
use std::path::{Path, PathBuf};

/// Version-control workspace context of a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceContext {
    /// No version control: multiple workspaces cannot exist, so coordination
    /// is inapplicable and callers proceed silently.
    NoVcs,
    /// The directory belongs to the clone's primary workspace.
    Primary,
    /// The directory belongs to a secondary workspace; `primary` is the
    /// primary workspace root.
    Secondary { primary: PathBuf },
    /// Version control is present but the primary workspace cannot be
    /// determined (the configured or fallback `default` jj workspace name is
    /// unknown to the repo) or detection is degraded (metadata present but
    /// unreadable); callers warn and proceed.
    Undeterminable,
}

/// Determine the workspace context of `dir`.
///
/// When both VCS markers are present, the innermost (owning) marker wins:
/// a `.jj` directory in the current tree means jj governs this checkout, and
/// colocated jj+git repos share one store, so jj-first is safe there. Plain
/// git trees keep git detection, which defines the main working tree as the
/// primary workspace. jj workspaces are peers, so the primary is the
/// workspace named by `[workspace] primary` in `gov/config.toml`, or jj's
/// initial `default` workspace when no primary is configured.
pub fn detect(dir: &Path, config: &Config) -> WorkspaceContext {
    if owning_vcs(dir) == Some(VcsKind::Jj) {
        return jj_context(dir, config);
    }
    if let Some(context) = git_context(dir) {
        return context;
    }
    jj_context(dir, config)
}

/// Version-control family hosting a clone's shared storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsKind {
    Git,
    Jj,
}

/// Shared VCS storage of the clone containing `dir`.
///
/// Implements [[RFC-0010:C-REGISTRY]]: `storage` is the rendezvous point
/// every workspace of the clone observes (the git common dir or the jj repo
/// dir), and `repo_root` is the current workspace root, used to derive the
/// registry namespace from the repo-relative governed root so every
/// workspace of one governed project shares one registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedStorage {
    pub kind: VcsKind,
    /// Directory shared by every workspace of the clone.
    pub storage: PathBuf,
    /// Canonical root of the workspace `dir` belongs to.
    pub repo_root: PathBuf,
}

/// Outcome of locating the clone's shared VCS storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageLookup {
    /// No version control: coordination is inapplicable, callers stay silent.
    NoVcs,
    /// Shared storage located.
    Found(SharedStorage),
    /// Version control is present but shared storage cannot be determined;
    /// callers degrade toward caution with a warning.
    Unavailable,
}

/// Locate the VCS shared storage of the clone containing `dir`.
pub fn shared_storage(dir: &Path) -> StorageLookup {
    // The owning (innermost) VCS marker wins, so a jj workspace nested in an
    // enclosing git tree is not misdetected as git — [[RFC-0010:C-REGISTRY]].
    if owning_vcs(dir) == Some(VcsKind::Jj) {
        return jj_shared_storage(dir);
    }
    if let Some(lookup) = git_shared_storage(dir) {
        return lookup;
    }
    jj_shared_storage(dir)
}

/// The VCS owning `dir`: the nearest ancestor marker wins, with jj preferred
/// when `.jj` and `.git` share a directory (a colocated repo shares one
/// store). Unreadable markers count as present, failing toward caution.
fn owning_vcs(dir: &Path) -> Option<VcsKind> {
    let mut current = Some(dir);
    while let Some(path) = current {
        if entry_present(&path.join(".jj")) {
            return Some(VcsKind::Jj);
        }
        if entry_present(&path.join(".git")) {
            return Some(VcsKind::Git);
        }
        current = path.parent();
    }
    None
}

fn git_shared_storage(dir: &Path) -> Option<StorageLookup> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--git-common-dir", "--show-toplevel"])
        .current_dir(dir)
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        // git missing, failed, or unconvinced while `.git` metadata exists:
        // fail toward caution rather than taking the silent no-VCS path.
        _ if has_ancestor_entry(dir, ".git") => return Some(StorageLookup::Unavailable),
        _ => return None,
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let (Some(common_dir), Some(repo_root)) = (lines.next(), lines.next()) else {
        return Some(StorageLookup::Unavailable);
    };
    let (Some(common_dir), Some(repo_root)) = (
        resolve_reported(dir, common_dir),
        resolve_reported(dir, repo_root),
    ) else {
        return Some(StorageLookup::Unavailable);
    };
    Some(StorageLookup::Found(SharedStorage {
        kind: VcsKind::Git,
        storage: common_dir,
        repo_root,
    }))
}

fn jj_shared_storage(dir: &Path) -> StorageLookup {
    let output = std::process::Command::new("jj")
        .args(["--ignore-working-copy", "root"])
        .current_dir(dir)
        .output();
    let root = match output {
        Ok(output) if output.status.success() => {
            let reported = String::from_utf8_lossy(&output.stdout);
            let Some(root) = resolve_reported(dir, reported.trim()) else {
                return StorageLookup::Unavailable;
            };
            root
        }
        // jj unavailable or unconvinced while `.jj` metadata exists: degrade.
        _ if has_ancestor_entry(dir, ".jj") => return StorageLookup::Unavailable,
        _ => return StorageLookup::NoVcs,
    };
    let store = root.join(".jj").join("repo");
    if store.is_dir() {
        return StorageLookup::Found(SharedStorage {
            kind: VcsKind::Jj,
            storage: store,
            repo_root: root,
        });
    }
    // A secondary jj workspace records `.jj/repo` as a text file pointing at
    // the shared repo store; the path is relative to the workspace's `.jj`
    // directory. Follow it so every workspace of the clone shares storage.
    if store.is_file()
        && let Ok(pointer) = std::fs::read_to_string(&store)
    {
        let pointer = Path::new(pointer.trim());
        let target = if pointer.is_absolute() {
            pointer.to_path_buf()
        } else {
            root.join(".jj").join(pointer)
        };
        if let Ok(store) = std::fs::canonicalize(&target)
            && store.is_dir()
        {
            return StorageLookup::Found(SharedStorage {
                kind: VcsKind::Jj,
                storage: store,
                repo_root: root,
            });
        }
    }
    StorageLookup::Unavailable
}

fn git_context(dir: &Path) -> Option<WorkspaceContext> {
    let output = std::process::Command::new("git")
        .args([
            "rev-parse",
            "--git-dir",
            "--git-common-dir",
            "--is-bare-repository",
        ])
        .current_dir(dir)
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        // git missing, failed, or unconvinced while `.git` metadata exists:
        // fail toward caution rather than taking the silent no-VCS path.
        _ if has_ancestor_entry(dir, ".git") => return Some(WorkspaceContext::Undeterminable),
        _ => return None,
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let (Some(git_dir), Some(common_dir), Some(is_bare)) =
        (lines.next(), lines.next(), lines.next())
    else {
        return Some(WorkspaceContext::Undeterminable);
    };
    if is_bare.trim() == "true" {
        // A bare repository has no working tree, so no main working tree can
        // be named; resolve toward caution — [[RFC-0010:C-COMMAND-SCOPE]].
        return Some(WorkspaceContext::Undeterminable);
    }
    let (Some(git_dir), Some(common_dir)) = (
        resolve_reported(dir, git_dir),
        resolve_reported(dir, common_dir),
    ) else {
        return Some(WorkspaceContext::Undeterminable);
    };
    if git_dir == common_dir {
        // Main working tree: the repository metadata lives in this tree.
        Some(WorkspaceContext::Primary)
    } else {
        // Linked worktree: the common dir is the main tree's `.git`. Bare
        // and separate-git-dir layouts fail this verification, resolving
        // toward caution instead of naming a bogus primary workspace.
        match common_dir.parent() {
            Some(primary)
                if std::fs::canonicalize(primary.join(".git")).ok() == Some(common_dir.clone()) =>
            {
                Some(WorkspaceContext::Secondary {
                    primary: primary.to_path_buf(),
                })
            }
            _ => Some(WorkspaceContext::Undeterminable),
        }
    }
}

fn jj_context(dir: &Path, config: &Config) -> WorkspaceContext {
    let output = std::process::Command::new("jj")
        .args(["--ignore-working-copy", "root"])
        .current_dir(dir)
        .output();
    let root = match output {
        Ok(output) if output.status.success() => {
            let reported = String::from_utf8_lossy(&output.stdout);
            let Some(root) = resolve_reported(dir, reported.trim()) else {
                return WorkspaceContext::Undeterminable;
            };
            root
        }
        // jj unavailable or unconvinced while `.jj` metadata exists: degrade.
        _ if has_ancestor_entry(dir, ".jj") => return WorkspaceContext::Undeterminable,
        _ => return WorkspaceContext::NoVcs,
    };
    match jj_workspaces(dir) {
        Some(workspaces) => classify_jj(config, &root, &workspaces),
        // The repo answered `jj root` but not `jj workspace list`: detection
        // is degraded, so resolve toward caution.
        None => WorkspaceContext::Undeterminable,
    }
}

/// jj workspace names with their canonical roots, as reported by
/// `jj workspace list`. A template with a tab separator keeps parsing exact
/// even for workspace names or roots containing spaces.
fn jj_workspaces(dir: &Path) -> Option<Vec<(String, PathBuf)>> {
    let output = std::process::Command::new("jj")
        .args([
            "--ignore-working-copy",
            "workspace",
            "list",
            "-T",
            r#"name ++ "\t" ++ root ++ "\n""#,
        ])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut workspaces = Vec::new();
    for line in stdout.lines() {
        let (name, root) = line.split_once('\t')?;
        // A workspace whose checkout was deleted still appears in the list;
        // skip it rather than failing the whole lookup.
        if let Ok(root) = std::fs::canonicalize(root) {
            workspaces.push((name.to_string(), root));
        }
    }
    Some(workspaces)
}

/// Classify the jj workspace rooted at `current_root`: the primary is the
/// configured workspace name, or jj's initial `default` workspace when
/// unconfigured — [[RFC-0010:C-COMMAND-SCOPE]].
fn classify_jj(
    config: &Config,
    current_root: &Path,
    workspaces: &[(String, PathBuf)],
) -> WorkspaceContext {
    let Some((current_name, _)) = workspaces.iter().find(|(_, root)| root == current_root) else {
        return WorkspaceContext::Undeterminable;
    };
    let primary_name = config.workspace.primary.as_deref().unwrap_or("default");
    if current_name == primary_name {
        return WorkspaceContext::Primary;
    }
    match workspaces.iter().find(|(name, _)| name == primary_name) {
        Some((_, primary)) => WorkspaceContext::Secondary {
            primary: primary.clone(),
        },
        // The configured or fallback primary name is unknown to the repo.
        None => WorkspaceContext::Undeterminable,
    }
}

/// Canonicalize a VCS-reported path, resolving it against `dir` when relative.
fn resolve_reported(dir: &Path, reported: &str) -> Option<PathBuf> {
    let path = Path::new(reported);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        dir.join(path)
    };
    std::fs::canonicalize(path).ok()
}

/// Whether `dir` or any ancestor contains an entry named `name`. Unreadable
/// entries count as present: undeterminable presence resolves toward caution.
fn has_ancestor_entry(dir: &Path, name: &str) -> bool {
    let mut current = Some(dir);
    while let Some(path) = current {
        if entry_present(&path.join(name)) {
            return true;
        }
        current = path.parent();
    }
    false
}

/// Whether `path` exists in any form. Unreadable entries count as present:
/// undeterminable presence resolves toward caution.
fn entry_present(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_available(tool: &str) -> bool {
        std::process::Command::new(tool)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn run(dir: &Path, tool: &str, args: &[&str]) {
        let output = std::process::Command::new(tool)
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{tool} {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn no_vcs_directory_is_silent_single_checkout() {
        let temp = tempfile::TempDir::new().unwrap();
        assert_eq!(
            detect(temp.path(), &Config::default()),
            WorkspaceContext::NoVcs
        );
        assert_eq!(shared_storage(temp.path()), StorageLookup::NoVcs);
    }

    #[test]
    fn git_main_worktree_is_primary() {
        if !tool_available("git") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        run(temp.path(), "git", &["init"]);
        assert_eq!(
            detect(temp.path(), &Config::default()),
            WorkspaceContext::Primary
        );
        assert_eq!(
            shared_storage(temp.path()),
            StorageLookup::Found(SharedStorage {
                kind: VcsKind::Git,
                storage: std::fs::canonicalize(temp.path().join(".git")).unwrap(),
                repo_root: std::fs::canonicalize(temp.path()).unwrap(),
            })
        );
    }

    #[test]
    fn git_linked_worktree_is_secondary() {
        if !tool_available("git") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        let main = temp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        run(&main, "git", &["init"]);
        run(&main, "git", &["config", "user.email", "test@example.com"]);
        run(&main, "git", &["config", "user.name", "Test"]);
        run(&main, "git", &["commit", "--allow-empty", "-m", "init"]);
        let secondary = temp.path().join("secondary");
        run(
            &main,
            "git",
            &[
                "worktree",
                "add",
                secondary.to_str().unwrap(),
                "-b",
                "secondary",
            ],
        );
        let expected_primary = std::fs::canonicalize(&main).unwrap();
        assert_eq!(
            detect(&secondary, &Config::default()),
            WorkspaceContext::Secondary {
                primary: expected_primary
            }
        );

        // Both workspaces resolve to the same shared storage, each with its
        // own workspace root.
        let common = std::fs::canonicalize(main.join(".git")).unwrap();
        assert_eq!(
            shared_storage(&secondary),
            StorageLookup::Found(SharedStorage {
                kind: VcsKind::Git,
                storage: common.clone(),
                repo_root: std::fs::canonicalize(&secondary).unwrap(),
            })
        );
        assert_eq!(
            shared_storage(&main),
            StorageLookup::Found(SharedStorage {
                kind: VcsKind::Git,
                storage: common,
                repo_root: std::fs::canonicalize(&main).unwrap(),
            })
        );
    }

    #[test]
    fn jj_default_workspace_is_primary_without_configuration() {
        if !tool_available("jj") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        run(
            temp.path(),
            "jj",
            &["--config", "git.colocate=false", "git", "init"],
        );
        // Unconfigured jj repos fall back to the initial `default` workspace —
        // [[RFC-0010:C-COMMAND-SCOPE]].
        assert_eq!(
            detect(temp.path(), &Config::default()),
            WorkspaceContext::Primary
        );
    }

    #[test]
    fn jj_added_workspace_is_secondary_to_default_without_configuration() {
        if !tool_available("jj") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        let main = temp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        init_jj_repo(&main);
        let secondary = temp.path().join("secondary");
        run(
            &main,
            "jj",
            &["workspace", "add", secondary.to_str().unwrap()],
        );
        assert_eq!(
            detect(&secondary, &Config::default()),
            WorkspaceContext::Secondary {
                primary: std::fs::canonicalize(&main).unwrap()
            }
        );
        assert_eq!(detect(&main, &Config::default()), WorkspaceContext::Primary);
    }

    #[test]
    fn jj_configured_primary_name_classifies_workspaces() {
        if !tool_available("jj") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        let main = temp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        init_jj_repo(&main);
        let other = temp.path().join("other");
        run(
            &main,
            "jj",
            &[
                "workspace",
                "add",
                "--name",
                "other",
                other.to_str().unwrap(),
            ],
        );

        let mut config = Config::default();
        config.workspace.primary = Some("other".to_string());
        assert_eq!(detect(&other, &config), WorkspaceContext::Primary);
        assert_eq!(
            detect(&main, &config),
            WorkspaceContext::Secondary {
                primary: std::fs::canonicalize(&other).unwrap()
            }
        );

        // A configured name unknown to the repo cannot be determined.
        config.workspace.primary = Some("nonexistent".to_string());
        assert_eq!(detect(&main, &config), WorkspaceContext::Undeterminable);
    }

    /// Initialize a non-colocated jj repo with a described initial change.
    fn init_jj_repo(dir: &Path) {
        run(
            dir,
            "jj",
            &["--config", "git.colocate=false", "git", "init"],
        );
        run(dir, "jj", &["describe", "-m", "init"]);
    }

    #[test]
    fn jj_secondary_workspace_shares_storage_through_repo_pointer() {
        if !tool_available("jj") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        let main = temp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        init_jj_repo(&main);
        let secondary = temp.path().join("secondary");
        run(
            &main,
            "jj",
            &["workspace", "add", secondary.to_str().unwrap()],
        );
        // The secondary records `.jj/repo` as a pointer file, not a directory.
        assert!(secondary.join(".jj/repo").is_file());

        let store = std::fs::canonicalize(main.join(".jj/repo")).unwrap();
        assert_eq!(
            shared_storage(&secondary),
            StorageLookup::Found(SharedStorage {
                kind: VcsKind::Jj,
                storage: store.clone(),
                repo_root: std::fs::canonicalize(&secondary).unwrap(),
            })
        );
        assert_eq!(
            shared_storage(&main),
            StorageLookup::Found(SharedStorage {
                kind: VcsKind::Jj,
                storage: store,
                repo_root: std::fs::canonicalize(&main).unwrap(),
            })
        );
    }

    #[test]
    fn jj_workspace_nested_in_git_tree_is_not_git_primary() {
        if !tool_available("git") || !tool_available("jj") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        run(temp.path(), "git", &["init"]);
        let inner = temp.path().join("inner");
        std::fs::create_dir(&inner).unwrap();
        init_jj_repo(&inner);

        // The innermost `.jj` marker owns the checkout; the enclosing git
        // tree must not mask it as a git primary workspace. The inner repo's
        // initial `default` workspace is its primary.
        assert_eq!(
            detect(&inner, &Config::default()),
            WorkspaceContext::Primary
        );
        assert_eq!(
            shared_storage(&inner),
            StorageLookup::Found(SharedStorage {
                kind: VcsKind::Jj,
                storage: std::fs::canonicalize(inner.join(".jj/repo")).unwrap(),
                repo_root: std::fs::canonicalize(&inner).unwrap(),
            })
        );
    }

    #[test]
    fn colocated_jj_git_repo_is_jj_governed() {
        if !tool_available("jj") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        run(temp.path(), "jj", &["git", "init"]);
        assert!(temp.path().join(".jj").exists() && temp.path().join(".git").exists());
        // Colocated repos share one store, so jj-first detection applies;
        // the unconfigured repo's initial `default` workspace is primary.
        assert_eq!(
            detect(temp.path(), &Config::default()),
            WorkspaceContext::Primary
        );
        match shared_storage(temp.path()) {
            StorageLookup::Found(storage) => assert_eq!(storage.kind, VcsKind::Jj),
            other => panic!("expected jj shared storage, got {other:?}"),
        }
    }

    #[test]
    fn bare_repo_and_its_worktree_are_undeterminable() {
        if !tool_available("git") {
            return;
        }
        let temp = tempfile::TempDir::new().unwrap();
        let main = temp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        run(&main, "git", &["init"]);
        run(&main, "git", &["config", "user.email", "test@example.com"]);
        run(&main, "git", &["config", "user.name", "Test"]);
        run(&main, "git", &["commit", "--allow-empty", "-m", "init"]);
        let bare = temp.path().join("bare.git");
        run(
            temp.path(),
            "git",
            &["clone", "--bare", "main", bare.to_str().unwrap()],
        );
        let worktree = temp.path().join("wt");
        run(
            &bare,
            "git",
            &["worktree", "add", worktree.to_str().unwrap()],
        );

        // The bare repo has no working tree, and its worktree's primary
        // cannot be named: both resolve toward caution instead of naming a
        // bogus primary path — [[RFC-0010:C-COMMAND-SCOPE]].
        assert_eq!(
            detect(&bare, &Config::default()),
            WorkspaceContext::Undeterminable
        );
        assert_eq!(
            detect(&worktree, &Config::default()),
            WorkspaceContext::Undeterminable
        );
    }
}
