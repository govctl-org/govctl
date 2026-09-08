//! Integration tests for [[RFC-0010:C-ARTIFACT-CLAIM]]: version-semantics
//! operations hold exclusive, expiring claims on the RFCs they mutate;
//! content edits on a claimed RFC warn without blocking; claims support
//! explicit release and audited takeover; and without version control
//! claims are silently inactive.

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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Initialize a git project whose committed tree already contains one draft
/// RFC, so linked worktrees branch off with the RFC present.
fn init_git_project_with_rfc() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = common::init_project()?;
    let dir = temp.path();
    run_tool(dir, "git", &["init"])?;
    run_tool(dir, "git", &["config", "user.email", "test@example.com"])?;
    run_tool(dir, "git", &["config", "user.name", "Test"])?;
    run_govctl(dir, &["rfc", "new", "Test RFC"])?;
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

/// Write a claim record directly, simulating another workspace's claim.
fn write_claim(
    main: &Path,
    id: &str,
    workspace: &Path,
    last_activity: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let claims = registry_dir(main)?.join("claims");
    std::fs::create_dir_all(&claims)?;
    std::fs::write(
        claims.join(format!("{id}.toml")),
        format!(
            "id = \"{id}\"\nworkspace = \"{}\"\nlast_activity = {last_activity}\n",
            workspace.display()
        ),
    )?;
    Ok(())
}

fn read_claim(main: &Path, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(
        registry_dir(main)?
            .join("claims")
            .join(format!("{id}.toml")),
    )?)
}

#[test]
fn foreign_live_claim_blocks_finalize_and_names_workspace() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    // Main's finalize acquires the claim on RFC-0001.
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;

    // The same version-semantics operation from the secondary workspace must
    // fail without mutating anything, naming the claiming workspace.
    let output = govctl(&secondary, &["rfc", "finalize", "RFC-0001", "normative"])?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("E0824"), "expected claim error: {stderr}");
    let main_root = std::fs::canonicalize(main.path())?;
    assert!(
        stderr.contains(&main_root.display().to_string()),
        "diagnostic must name the claiming workspace: {stderr}"
    );
    let rfc_toml = std::fs::read_to_string(secondary.join("gov/rfc/RFC-0001/rfc.toml"))?;
    assert!(
        rfc_toml.contains("status = \"draft\""),
        "blocked finalize must not mutate the RFC: {rfc_toml}"
    );
    Ok(())
}

#[test]
fn content_edits_warn_but_succeed_on_claimed_rfc() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;
    let main_root = std::fs::canonicalize(main.path())?;

    let output = run_govctl(
        &secondary,
        &["rfc", "edit", "RFC-0001", "title", "--set", "Edited"],
    )?;
    let edit_stderr = stderr(&output);
    assert!(
        edit_stderr.contains("W0116"),
        "expected claim warning: {edit_stderr}"
    );
    assert!(
        edit_stderr.contains(&main_root.display().to_string()),
        "warning must name the claiming workspace: {edit_stderr}"
    );
    assert!(
        std::fs::read_to_string(secondary.join("gov/rfc/RFC-0001/rfc.toml"))?
            .contains("title = \"Edited\""),
        "content edit must succeed despite the foreign claim"
    );

    // Clause authoring on the claimed RFC warns without blocking too. The
    // empty `clauses/` dir is not tracked by git, so the worktree checkout
    // lacks it; recreate it as content creation would.
    std::fs::create_dir_all(secondary.join("gov/rfc/RFC-0001/clauses"))?;
    let output = run_govctl(&secondary, &["clause", "new", "RFC-0001:C-SCOPE", "Scope"])?;
    let clause_stderr = stderr(&output);
    assert!(
        clause_stderr.contains("W0116"),
        "expected claim warning for clause authoring: {clause_stderr}"
    );
    assert!(
        secondary
            .join("gov/rfc/RFC-0001/clauses/C-SCOPE.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn supersede_requires_claims_on_both_rfcs() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    run_govctl(main.path(), &["rfc", "new", "Replacement RFC"])?;
    run_tool(main.path(), "git", &["add", "."])?;
    run_tool(main.path(), "git", &["commit", "-m", "replacement"])?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;

    // Main claims RFC-0001; the secondary claims RFC-0002.
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;
    run_govctl(&secondary, &["rfc", "finalize", "RFC-0002", "normative"])?;

    // Supersession mutates both RFCs, so the foreign claim on the
    // replacement blocks it before any mutation.
    let output = govctl(
        main.path(),
        &[
            "rfc",
            "supersede",
            "RFC-0001",
            "--by",
            "RFC-0002",
            "--force",
        ],
    )?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("E0824"), "expected claim error: {stderr}");
    let secondary_root = std::fs::canonicalize(&secondary)?;
    assert!(
        stderr.contains(&secondary_root.display().to_string()),
        "diagnostic must name the claiming workspace: {stderr}"
    );
    let rfc_toml = std::fs::read_to_string(main.path().join("gov/rfc/RFC-0001/rfc.toml"))?;
    assert!(
        rfc_toml.contains("status = \"normative\""),
        "blocked supersede must not mutate the superseded RFC: {rfc_toml}"
    );

    // After taking over the replacement's claim, supersession proceeds.
    run_govctl(main.path(), &["claim", "steal", "RFC-0002"])?;
    run_govctl(
        main.path(),
        &[
            "rfc",
            "supersede",
            "RFC-0001",
            "--by",
            "RFC-0002",
            "--force",
        ],
    )?;
    assert!(
        std::fs::read_to_string(main.path().join("gov/rfc/RFC-0001/rfc.toml"))?
            .contains("status = \"deprecated\"")
    );
    Ok(())
}

#[test]
fn expired_claim_does_not_block() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    let holder = TempDir::new()?;
    // A claim from another workspace whose last activity is far beyond the
    // 7-day default inactivity period is expired and must not block.
    write_claim(
        main.path(),
        "RFC-0001",
        holder.path(),
        now_secs() - 30 * 24 * 60 * 60,
    )?;

    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;

    // The expired claim was re-acquired by the finalizing workspace.
    let claim = read_claim(main.path(), "RFC-0001")?;
    let main_root = std::fs::canonicalize(main.path())?;
    assert!(
        claim.contains(&main_root.display().to_string()),
        "expected the claim to be re-acquired by main: {claim}"
    );
    Ok(())
}

#[test]
fn missing_workspace_claim_does_not_block() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    // The owning workspace path no longer exists: the claim is expired even
    // with a fresh timestamp.
    write_claim(
        main.path(),
        "RFC-0001",
        Path::new("/nonexistent/govctl-test-workspace"),
        now_secs(),
    )?;
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;
    Ok(())
}

#[test]
fn release_frees_claim_for_other_workspaces() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;

    // Another workspace cannot release a claim it does not hold.
    let output = govctl(&secondary, &["claim", "release", "RFC-0001"])?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("E0824"));

    // The holding workspace releases; the claim record is gone.
    run_govctl(main.path(), &["claim", "release", "RFC-0001"])?;
    assert!(
        !registry_dir(main.path())?
            .join("claims")
            .join("RFC-0001.toml")
            .exists()
    );

    // The secondary workspace can now finalize.
    run_govctl(&secondary, &["rfc", "finalize", "RFC-0001", "normative"])?;
    Ok(())
}

#[test]
fn steal_transfers_claim_and_records_audit_event() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    let holder = TempDir::new()?;
    let secondary = add_worktree(main.path(), &holder, "secondary")?;
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;

    run_govctl(&secondary, &["claim", "steal", "RFC-0001"])?;

    // The claim now belongs to the secondary workspace.
    let claim = read_claim(main.path(), "RFC-0001")?;
    let secondary_root = std::fs::canonicalize(&secondary)?;
    assert!(
        claim.contains(&secondary_root.display().to_string()),
        "claim must name the new owner: {claim}"
    );

    // The takeover is recorded as an audit event naming both workspaces.
    let audit_dir = registry_dir(main.path())?.join("audit");
    let events: Vec<PathBuf> = std::fs::read_dir(&audit_dir)?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    assert_eq!(events.len(), 1, "expected one audit event in {audit_dir:?}");
    let event = std::fs::read_to_string(&events[0])?;
    let main_root = std::fs::canonicalize(main.path())?;
    assert!(event.contains("takeover"), "{event}");
    assert!(event.contains(&main_root.display().to_string()), "{event}");
    assert!(
        event.contains(&secondary_root.display().to_string()),
        "{event}"
    );

    // The new claim holder can perform version-semantics operations.
    run_govctl(&secondary, &["rfc", "finalize", "RFC-0001", "normative"])?;
    Ok(())
}

#[test]
fn claim_list_shows_live_and_expired_claims() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;

    let output = run_govctl(main.path(), &["claim", "list"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RFC-0001"), "{stdout}");
    assert!(stdout.contains("live"), "{stdout}");
    Ok(())
}

#[test]
fn claim_list_warns_on_corrupt_record_but_succeeds() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;

    // An undecodable claim record must not vanish silently —
    // [[RFC-0010:C-REGISTRY]]: the read-only listing skips it with a
    // corruption warning and still succeeds.
    let claims = registry_dir(main.path())?.join("claims");
    std::fs::write(claims.join("RFC-0099.toml"), "not = [valid")?;

    let output = govctl(main.path(), &["claim", "list"])?;
    assert!(
        output.status.success(),
        "claim list must succeed despite the corrupt record: {}",
        stderr(&output)
    );
    let stderr = stderr(&output);
    assert!(
        stderr.contains("W0115"),
        "expected a corruption warning: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("RFC-0001"),
        "healthy records must still be listed: {stdout}"
    );
    assert!(
        !stdout.contains("RFC-0099"),
        "the corrupt record is skipped: {stdout}"
    );
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
fn claim_list_does_not_block_on_held_allocation_lock() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project_with_rfc()?;
    run_govctl(main.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;

    // [[RFC-0010:C-REGISTRY]]: a read-only claim listing must neither block
    // on nor degrade behind a concurrently held allocation lock.
    let _lock = hold_allocation_lock(main.path())?;
    let started = std::time::Instant::now();
    let output = govctl(main.path(), &["claim", "list"])?;
    assert!(
        output.status.success(),
        "claim list failed under a held allocation lock: {}",
        stderr(&output)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RFC-0001"), "{stdout}");
    assert!(
        !stderr(&output).contains("W0115"),
        "claim list must not degrade behind a held lock: {}",
        stderr(&output)
    );
    // The default lock timeout is 30s; a blocking reader would hit it.
    assert!(
        started.elapsed() < std::time::Duration::from_secs(25),
        "claim list blocked on the held allocation lock"
    );
    Ok(())
}

#[test]
fn no_vcs_claims_are_silently_inactive() -> TestResult {
    let temp = common::init_project()?;
    run_govctl(temp.path(), &["rfc", "new", "Test RFC"])?;
    let output = run_govctl(temp.path(), &["rfc", "finalize", "RFC-0001", "normative"])?;
    assert!(
        !stderr(&output).contains("warning"),
        "no claim warning expected without version control: {}",
        stderr(&output)
    );
    let output = run_govctl(temp.path(), &["claim", "list"])?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(combined.contains("No artifact claims"), "{combined}");
    assert!(!temp.path().join(".git").exists());
    Ok(())
}
