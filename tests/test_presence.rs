//! Integration tests for [[RFC-0010:C-PRESENCE]]: activating a work item (or
//! creating one active) registers an advisory presence record in the shared
//! coordination registry; `govctl status` surfaces live presence records
//! owned by other workspaces; presence follows the claim liveness rules and
//! never blocks operations.

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

fn run_govctl(dir: &Path, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    let output = govctl(dir, args)?;
    assert!(
        output.status.success(),
        "govctl {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Initialize a git-backed govctl project and return it.
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

/// The clone's single per-project registry directory.
fn registry_dir(main: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let base = main.join(".git/govctl/registry");
    let dirs: Vec<PathBuf> = std::fs::read_dir(&base)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(dirs.len(), 1, "expected one registry under {base:?}");
    Ok(dirs.into_iter().next().ok_or("registry missing")?)
}

/// Write a presence record directly, simulating another workspace's record.
fn write_presence(
    main: &Path,
    id: &str,
    title: &str,
    workspace: &Path,
    last_activity: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let presence = registry_dir(main)?.join("presence");
    std::fs::create_dir_all(&presence)?;
    std::fs::write(
        presence.join(format!("{id}.toml")),
        format!(
            "id = \"{id}\"\ntitle = \"{title}\"\nworkspace = \"{}\"\nlast_activity = {last_activity}\n",
            workspace.display()
        ),
    )?;
    Ok(())
}

/// Extract the first work item id from command output (the id is reported on
/// stderr via `ui::sub_info`).
fn work_id_of(output: &Output) -> Result<String, Box<dyn std::error::Error>> {
    let combined = format!("{}{}", stdout(output), stderr(output));
    let id = combined
        .split_whitespace()
        .find(|token| token.starts_with("WI-"))
        .ok_or("no work item id in output")?;
    Ok(id
        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-')
        .to_string())
}

/// The id of the work item created by `work new --active` in `dir`.
fn activated_work_id(dir: &Path, title: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = run_govctl(dir, &["work", "new", title, "--active"])?;
    work_id_of(&output)
}

#[test]
fn activation_registers_presence_visible_from_other_workspaces() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    let work_id = activated_work_id(&secondary, "Presence probe")?;

    // The record exists in the shared registry, owned by the secondary.
    let record = std::fs::read_to_string(
        registry_dir(main.path())?
            .join("presence")
            .join(format!("{work_id}.toml")),
    )?;
    let secondary_root = std::fs::canonicalize(&secondary)?;
    assert!(
        record.contains(&secondary_root.display().to_string()),
        "presence record must name the owning workspace: {record}"
    );

    // Main's status overlay lists the item with id, title, and workspace.
    let output = run_govctl(main.path(), &["status"])?;
    let out = stdout(&output);
    assert!(
        out.contains("Active in Other Workspaces"),
        "expected the presence overlay: {out}"
    );
    assert!(
        out.contains(&work_id),
        "overlay must name the work item: {out}"
    );
    assert!(
        out.contains("Presence probe"),
        "overlay must name the title: {out}"
    );
    assert!(
        out.contains(&secondary_root.display().to_string()),
        "overlay must name the owning workspace: {out}"
    );

    // The secondary's own status does not list its own presence.
    let output = run_govctl(&secondary, &["status"])?;
    assert!(
        !stdout(&output).contains("Active in Other Workspaces"),
        "own presence must not appear in the overlay"
    );
    Ok(())
}

#[test]
fn move_to_active_registers_presence() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    // Created in queue status: no presence record yet.
    let output = run_govctl(&secondary, &["work", "new", "Queued probe"])?;
    let work_id = work_id_of(&output)?;
    assert!(
        !registry_dir(main.path())?
            .join("presence")
            .join(format!("{work_id}.toml"))
            .exists(),
        "queue-status creation must not register presence"
    );

    run_govctl(&secondary, &["work", "move", &work_id, "active"])?;
    assert!(
        registry_dir(main.path())?
            .join("presence")
            .join(format!("{work_id}.toml"))
            .exists(),
        "moving to active must register presence"
    );
    Ok(())
}

#[test]
fn single_checkout_status_output_has_no_overlay() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    activated_work_id(main.path(), "Local probe")?;

    let output = run_govctl(main.path(), &["status"])?;
    let out = stdout(&output);
    assert!(
        !out.contains("Active in Other Workspaces"),
        "single-checkout status must not show the overlay: {out}"
    );
    assert!(
        out.contains("Active Work"),
        "local active work still appears: {out}"
    );
    assert!(
        !stderr(&output).contains("warning"),
        "no warning expected in the healthy single-checkout case: {}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn presence_removed_when_work_item_leaves_active() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    let work_id = activated_work_id(&secondary, "Transient probe")?;
    let record_path = registry_dir(main.path())?
        .join("presence")
        .join(format!("{work_id}.toml"));
    assert!(record_path.exists());

    // Complete the work item in the secondary; the presence record is removed.
    run_govctl(
        &secondary,
        &[
            "work",
            "edit",
            &work_id,
            "acceptance_criteria",
            "--add",
            "fixed: done",
        ],
    )?;
    run_govctl(
        &secondary,
        &[
            "work",
            "edit",
            &work_id,
            "acceptance_criteria[0]",
            "--tick",
            "done",
        ],
    )?;
    run_govctl(&secondary, &["work", "move", &work_id, "done"])?;
    assert!(
        !record_path.exists(),
        "leaving active status must remove the presence record"
    );

    let output = run_govctl(main.path(), &["status"])?;
    assert!(
        !stdout(&output).contains("Active in Other Workspaces"),
        "completed work must not appear in the overlay"
    );
    Ok(())
}

#[test]
fn cross_workspace_completion_removes_presence_record() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;

    // Workspace A (main) activates the work item and commits it so another
    // worktree can check it out.
    let work_id = activated_work_id(main.path(), "Cross probe")?;
    run_tool(main.path(), "git", &["add", "."])?;
    run_tool(main.path(), "git", &["commit", "-m", "activate"])?;
    let record_path = registry_dir(main.path())?
        .join("presence")
        .join(format!("{work_id}.toml"));
    assert!(record_path.exists(), "activation must register presence");

    // Workspace B completes the item. [[RFC-0010:C-PRESENCE]]: leaving active
    // status removes the presence record whichever workspace owns it — A's
    // record must not stay live until expiry.
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;
    run_govctl(
        &secondary,
        &[
            "work",
            "edit",
            &work_id,
            "acceptance_criteria",
            "--add",
            "fixed: done",
        ],
    )?;
    run_govctl(
        &secondary,
        &[
            "work",
            "edit",
            &work_id,
            "acceptance_criteria[0]",
            "--tick",
            "done",
        ],
    )?;
    run_govctl(&secondary, &["work", "move", &work_id, "done"])?;
    assert!(
        !record_path.exists(),
        "completion in another workspace must remove the presence record"
    );

    // Neither workspace's status shows the item as active elsewhere.
    for dir in [main.path(), secondary.as_path()] {
        let output = run_govctl(dir, &["status"])?;
        assert!(
            !stdout(&output).contains("Active in Other Workspaces"),
            "completed work must not appear in either workspace's overlay: {}",
            stdout(&output)
        );
    }
    Ok(())
}

#[test]
fn status_warns_on_corrupt_presence_record_but_succeeds() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    // Seed the registry via an ordinary artifact creation.
    run_govctl(main.path(), &["work", "new", "Seed probe"])?;

    // An undecodable presence record must not vanish silently —
    // [[RFC-0010:C-REGISTRY]]: the read-only overlay skips it with a
    // corruption warning and status still succeeds.
    let presence = registry_dir(main.path())?.join("presence");
    std::fs::create_dir_all(&presence)?;
    std::fs::write(presence.join("WI-2026-09-08-904.toml"), "not = [valid")?;

    let output = govctl(main.path(), &["status"])?;
    assert!(
        output.status.success(),
        "status must succeed despite the corrupt record: {}",
        stderr(&output)
    );
    let stderr = stderr(&output);
    assert!(
        stderr.contains("W0115"),
        "expected a corruption warning: {stderr}"
    );
    assert!(
        !stdout(&output).contains("Active in Other Workspaces"),
        "the corrupt record must not appear in the overlay"
    );
    Ok(())
}

#[test]
fn expired_and_missing_workspace_records_are_hidden() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;

    // Seed the registry via an ordinary artifact creation.
    run_govctl(main.path(), &["work", "new", "Seed probe"])?;

    // Expired by inactivity (far beyond the 7-day default).
    write_presence(
        main.path(),
        "WI-2026-09-08-901",
        "Stale probe",
        holder.path(),
        now_secs() - 30 * 24 * 60 * 60,
    )?;
    // Owning workspace no longer exists, even with a fresh timestamp.
    write_presence(
        main.path(),
        "WI-2026-09-08-902",
        "Orphaned probe",
        Path::new("/nonexistent/govctl-test-workspace"),
        now_secs(),
    )?;

    let output = run_govctl(main.path(), &["status"])?;
    let out = stdout(&output);
    assert!(
        !out.contains("Active in Other Workspaces"),
        "expired and orphaned presence records must be hidden: {out}"
    );
    Ok(())
}

#[test]
fn no_vcs_presence_is_silently_inactive() -> TestResult {
    let temp = common::init_project()?;
    activated_work_id(temp.path(), "Silent probe")?;
    let output = run_govctl(temp.path(), &["status"])?;
    assert!(
        !stdout(&output).contains("Active in Other Workspaces"),
        "no overlay without version control"
    );
    assert!(
        !stderr(&output).contains("warning"),
        "no warning expected without version control: {}",
        stderr(&output)
    );
    assert!(!temp.path().join(".git").exists());
    Ok(())
}

/// Exclusively hold the clone's allocation lock until the returned file is
/// dropped, simulating a concurrent writer in another process.
fn hold_allocation_lock(main: &Path) -> Result<std::fs::File, Box<dyn std::error::Error>> {
    use fs2::FileExt;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(registry_dir(main)?.join("allocation.lock"))?;
    lock.try_lock_exclusive()?;
    Ok(lock)
}

#[test]
fn status_overlay_survives_held_allocation_lock() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;
    let work_id = activated_work_id(&secondary, "Locked probe")?;

    // [[RFC-0010:C-REGISTRY]]: the status overlay reads the registry without
    // the allocation lock, so a concurrent writer cannot block or hide it.
    let _lock = hold_allocation_lock(main.path())?;
    let output = run_govctl(main.path(), &["status"])?;
    let out = stdout(&output);
    assert!(
        out.contains("Active in Other Workspaces"),
        "the overlay must survive a held allocation lock: {out}"
    );
    assert!(
        out.contains(&work_id),
        "overlay must name the work item: {out}"
    );
    assert!(
        !stderr(&output).contains("W0115"),
        "status must not degrade behind a held lock: {}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn status_does_not_refresh_presence_records() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    // Seed the registry via an ordinary artifact creation.
    run_govctl(main.path(), &["work", "new", "Seed probe"])?;

    // A presence record owned by this workspace with a stale timestamp:
    // read-only status must not rewrite it (read sessions never write).
    let workspace = std::fs::canonicalize(main.path())?;
    write_presence(
        main.path(),
        "WI-2026-09-08-903",
        "Own probe",
        &workspace,
        now_secs() - 1000,
    )?;
    let record_path = registry_dir(main.path())?
        .join("presence")
        .join("WI-2026-09-08-903.toml");
    let before = std::fs::read_to_string(&record_path)?;
    run_govctl(main.path(), &["status"])?;
    let after = std::fs::read_to_string(&record_path)?;
    assert_eq!(
        before, after,
        "read-only status must not refresh presence records"
    );
    Ok(())
}
