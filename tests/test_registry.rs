//! Integration tests for [[RFC-0010:C-REGISTRY]] and
//! [[RFC-0010:C-ID-RESERVATION]]: workspaces of one clone never allocate the
//! same artifact ID, allocation witnesses shared version-control history,
//! no-VCS directories behave as before, and registry failure or corruption
//! degrades to single-checkout behavior with a warning.

mod common;

use common::TestResult;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn govctl(dir: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()
}

fn tool_available(tool_name: &str) -> bool {
    Command::new(tool_name)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn run_tool(dir: &Path, tool_name: &str, args: &[&str]) -> TestResult {
    let output = Command::new(tool_name)
        .args(args)
        .current_dir(dir)
        .output()?;
    assert!(
        output.status.success(),
        "{tool_name} {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// Initialize a govctl project inside a git repository with one commit, so
/// linked worktrees can be added.
fn init_git_project() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = common::init_project()?;
    let dir = temp.path();
    run_tool(dir, "git", &["init"])?;
    run_tool(dir, "git", &["config", "user.email", "test@example.com"])?;
    run_tool(dir, "git", &["config", "user.name", "Test"])?;
    run_tool(dir, "git", &["add", "."])?;
    run_tool(dir, "git", &["commit", "-m", "init"])?;
    Ok(temp)
}

/// Add a linked worktree on a new branch and return its path.
fn add_worktree(
    main: &Path,
    holder: &TempDir,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let secondary = holder.path().join(name);
    run_tool(
        main,
        "git",
        &[
            "worktree",
            "add",
            secondary.to_str().ok_or("non-utf8 worktree path")?,
            "-b",
            name,
        ],
    )?;
    Ok(secondary)
}

fn run_govctl(dir: &Path, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    let output = govctl(dir, args)?;
    assert!(
        output.status.success(),
        "govctl {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Registry directories under the clone's shared git storage.
fn registry_dirs(main: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let base = main.join(".git/govctl/registry");
    match std::fs::read_dir(base) {
        Ok(entries) => Ok(entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(err) => Err(err),
    }
}

/// Exclusively hold the clone's allocation lock until the returned file is
/// dropped, simulating a concurrent writer in another process.
fn hold_allocation_lock(main: &Path) -> Result<std::fs::File, Box<dyn std::error::Error>> {
    use fs2::FileExt;
    let registry = registry_dirs(main)?
        .into_iter()
        .next()
        .ok_or("registry must exist after seeding")?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(registry.join("allocation.lock"))?;
    lock.try_lock_exclusive()?;
    Ok(lock)
}

#[test]
fn workspaces_of_one_clone_allocate_distinct_ids() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    // The secondary worktree shares the committed tree (no artifacts yet);
    // without the registry both workspaces would allocate the same IDs.
    run_govctl(main.path(), &["adr", "new", "Main ADR"])?;
    run_govctl(main.path(), &["rfc", "new", "Main RFC"])?;
    run_govctl(main.path(), &["work", "new", "Main work"])?;

    run_govctl(&secondary, &["adr", "new", "Secondary ADR"])?;
    run_govctl(&secondary, &["rfc", "new", "Secondary RFC"])?;
    run_govctl(&secondary, &["work", "new", "Secondary work"])?;

    assert!(main.path().join("gov/adr/ADR-0001-main-adr.toml").exists());
    assert!(
        secondary
            .join("gov/adr/ADR-0002-secondary-adr.toml")
            .exists()
    );
    assert!(main.path().join("gov/rfc/RFC-0001").is_dir());
    assert!(secondary.join("gov/rfc/RFC-0002").is_dir());

    let date = common::today();
    assert!(
        main.path()
            .join(format!("gov/work/{date}-main-work.toml"))
            .exists()
    );
    let secondary_work =
        std::fs::read_to_string(secondary.join(format!("gov/work/{date}-secondary-work.toml")))?;
    let main_id = common::work_id(&date, 1);
    let secondary_id = common::work_id(&date, 2);
    assert!(
        secondary_work.contains(&format!("id = \"{secondary_id}\"")),
        "secondary work item must reserve {secondary_id}: {secondary_work}"
    );
    assert_ne!(main_id, secondary_id);

    // The registry lives in shared git storage, outside any working tree.
    assert_eq!(
        registry_dirs(main.path())?.len(),
        1,
        "expected one per-project registry under .git/govctl/registry"
    );
    assert!(!main.path().join("gov/govctl").exists());
    Ok(())
}

#[test]
fn allocation_witnesses_merged_and_deleted_history() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    // The secondary worktree branches off before any artifact exists, so its
    // tree stays stale for the whole test.
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    // In the main workspace: create ADR-0001, commit it, then delete it.
    run_govctl(main.path(), &["adr", "new", "Deleted ADR"])?;
    run_tool(main.path(), "git", &["add", "."])?;
    run_tool(main.path(), "git", &["commit", "-m", "add adr"])?;
    std::fs::remove_file(main.path().join("gov/adr/ADR-0001-deleted-adr.toml"))?;
    run_tool(main.path(), "git", &["add", "."])?;
    run_tool(main.path(), "git", &["commit", "-m", "delete adr"])?;

    // The stale secondary sees neither the ADR file nor a live reservation,
    // but shared history witnesses ADR-0001: it must not be reused.
    let output = run_govctl(&secondary, &["adr", "new", "Secondary ADR"])?;
    assert!(
        !stderr(&output).contains("warning"),
        "no degradation expected: {}",
        stderr(&output)
    );
    assert!(
        secondary
            .join("gov/adr/ADR-0002-secondary-adr.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn no_vcs_allocation_is_silent_and_creates_no_registry() -> TestResult {
    let temp = common::init_project()?;
    let output = run_govctl(temp.path(), &["adr", "new", "Plain ADR"])?;
    assert!(
        !stderr(&output).contains("warning"),
        "no registry warning expected without version control: {}",
        stderr(&output)
    );
    assert!(temp.path().join("gov/adr/ADR-0001-plain-adr.toml").exists());
    // Without version control there is no shared storage to host a registry.
    assert!(!temp.path().join(".git").exists());
    Ok(())
}

#[test]
fn concurrent_allocations_across_workspaces_never_collide() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    // Race allocations from both workspaces; the allocation lock must
    // serialize them so each gets a distinct ID.
    let mut children = Vec::new();
    for (dir, title) in [
        (main.path().to_path_buf(), "Race A"),
        (secondary.clone(), "Race B"),
        (main.path().to_path_buf(), "Race C"),
        (secondary.clone(), "Race D"),
    ] {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_govctl"))
                .args(["adr", "new", title])
                .current_dir(dir)
                .env("NO_COLOR", "1")
                .env("GOVCTL_DEFAULT_OWNER", "@test-user")
                .spawn()?,
        );
    }
    for child in children {
        let output = child.wait_with_output()?;
        assert!(
            output.status.success(),
            "concurrent adr new failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut ids = std::collections::BTreeSet::new();
    for dir in [main.path(), &secondary] {
        for entry in std::fs::read_dir(dir.join("gov/adr"))?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(id) = name.strip_suffix(".toml").and_then(|n| n.split('-').nth(1)) {
                ids.insert(format!("ADR-{id}"));
            }
        }
    }
    assert_eq!(ids.len(), 4, "expected four distinct ADR IDs: {ids:?}");
    Ok(())
}

#[test]
fn unwritable_shared_storage_degrades_with_warning() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    // A regular file where the registry directory must be created makes
    // every registry write fail while leaving version control intact.
    std::fs::write(main.path().join(".git/govctl"), b"blocked")?;

    let output = run_govctl(main.path(), &["adr", "new", "Degraded ADR"])?;
    let stderr = stderr(&output);
    assert!(
        stderr.contains("W0115"),
        "expected registry degradation warning: {stderr}"
    );
    // The command still succeeds with single-checkout allocation.
    assert!(
        main.path()
            .join("gov/adr/ADR-0001-degraded-adr.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn corrupt_reservation_degrades_with_warning() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    run_govctl(main.path(), &["adr", "new", "First ADR"])?;

    let registry = registry_dirs(main.path())?
        .into_iter()
        .next()
        .ok_or("registry must exist after the first reservation")?;
    std::fs::write(registry.join("reservations/corrupt.toml"), "not = [valid")?;

    let output = run_govctl(main.path(), &["adr", "new", "Second ADR"])?;
    let stderr = stderr(&output);
    assert!(
        stderr.contains("W0115"),
        "expected registry corruption warning: {stderr}"
    );
    // Degraded allocation still advances past the local tree's ADR-0001.
    assert!(
        main.path()
            .join("gov/adr/ADR-0002-second-adr.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn read_only_commands_do_not_block_on_held_allocation_lock() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    // Seed the registry so the allocation lock file exists.
    run_govctl(main.path(), &["adr", "new", "Seed ADR"])?;
    let _lock = hold_allocation_lock(main.path())?;

    // [[RFC-0010:C-REGISTRY]]: with the allocation lock held by another
    // process, concurrent status runs must neither block on the lock nor
    // degrade with a registry warning.
    let started = std::time::Instant::now();
    let mut children = Vec::new();
    for _ in 0..2 {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_govctl"))
                .args(["status"])
                .current_dir(main.path())
                .env("NO_COLOR", "1")
                .env("GOVCTL_DEFAULT_OWNER", "@test-user")
                .spawn()?,
        );
    }
    for child in children {
        let output = child.wait_with_output()?;
        assert!(
            output.status.success(),
            "status failed under a held allocation lock: {}",
            stderr(&output)
        );
        assert!(
            !stderr(&output).contains("W0115"),
            "status must not degrade behind a held lock: {}",
            stderr(&output)
        );
    }
    // The default lock timeout is 30s; blocking readers would hit it.
    assert!(
        started.elapsed() < std::time::Duration::from_secs(25),
        "status blocked on the held allocation lock"
    );
    Ok(())
}

#[test]
fn reservation_survives_uncommitted_artifact_deletion() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    run_govctl(main.path(), &["adr", "new", "Doomed ADR"])?;

    // Deleting the artifact without committing leaves no history witness;
    // [[RFC-0010:C-ID-RESERVATION]] forbids discarding the reservation, so
    // the ID must not be reallocated.
    std::fs::remove_file(main.path().join("gov/adr/ADR-0001-doomed-adr.toml"))?;
    let output = run_govctl(main.path(), &["adr", "new", "Next ADR"])?;
    assert!(
        !stderr(&output).contains("warning"),
        "no degradation expected: {}",
        stderr(&output)
    );
    assert!(
        main.path().join("gov/adr/ADR-0002-next-adr.toml").exists(),
        "the unwitnessed ID must not be reallocated"
    );
    let registry = registry_dirs(main.path())?
        .into_iter()
        .next()
        .ok_or("registry must exist")?;
    assert!(
        registry.join("reservations/ADR-0001.toml").exists(),
        "the unwitnessed reservation must be kept"
    );
    Ok(())
}

#[test]
fn witnessed_reservation_is_pruned_after_committed_deletion() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    run_govctl(main.path(), &["adr", "new", "Recorded ADR"])?;
    run_tool(main.path(), "git", &["add", "."])?;
    run_tool(main.path(), "git", &["commit", "-m", "add adr"])?;

    // Once the artifact is recorded in shared history, deleting it lets the
    // reservation be pruned; the history witness still blocks reuse.
    std::fs::remove_file(main.path().join("gov/adr/ADR-0001-recorded-adr.toml"))?;
    run_govctl(main.path(), &["adr", "new", "Next ADR"])?;
    assert!(main.path().join("gov/adr/ADR-0002-next-adr.toml").exists());
    let registry = registry_dirs(main.path())?
        .into_iter()
        .next()
        .ok_or("registry must exist")?;
    assert!(
        !registry.join("reservations/ADR-0001.toml").exists(),
        "the witnessed reservation must be pruned"
    );
    Ok(())
}

#[test]
fn jj_secondary_workspace_reserves_distinct_ids() -> TestResult {
    if !tool_available("jj") {
        return Ok(());
    }
    let main = common::init_project()?;
    run_tool(
        main.path(),
        "jj",
        &["--config", "git.colocate=false", "git", "init"],
    )?;
    // Commit the gov tree so the secondary workspace checks it out.
    run_tool(main.path(), "jj", &["describe", "-m", "init"])?;
    run_tool(main.path(), "jj", &["new"])?;
    let holder = TempDir::new()?;
    let secondary = holder.path().join("secondary");
    run_tool(
        main.path(),
        "jj",
        &[
            "workspace",
            "add",
            secondary.to_str().ok_or("non-utf8 workspace path")?,
        ],
    )?;
    // A secondary jj workspace records `.jj/repo` as a pointer file; the
    // registry must follow it to the shared repo store — [[RFC-0010:C-REGISTRY]].
    assert!(secondary.join(".jj/repo").is_file());

    let output = run_govctl(&secondary, &["adr", "new", "Secondary ADR"])?;
    assert!(
        !stderr(&output).contains("W0115"),
        "the registry must be operative in a jj secondary workspace: {}",
        stderr(&output)
    );
    run_govctl(main.path(), &["adr", "new", "Main ADR"])?;

    assert!(
        secondary
            .join("gov/adr/ADR-0001-secondary-adr.toml")
            .exists(),
        "secondary allocation failed"
    );
    assert!(
        main.path().join("gov/adr/ADR-0002-main-adr.toml").exists(),
        "main must observe the secondary's reservation through shared storage"
    );
    // The reservation lives in the clone's shared jj repo store.
    let store = main.path().join(".jj/repo/govctl/registry");
    let registries: Vec<PathBuf> = std::fs::read_dir(&store)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(registries.len(), 1, "expected one registry under {store:?}");
    assert!(
        registries[0].join("reservations/ADR-0001.toml").exists(),
        "the secondary's reservation must be in shared storage"
    );
    Ok(())
}
