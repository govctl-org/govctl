//! Integration tests for [[RFC-0010:C-COMMAND-SCOPE]] trunk enforcement:
//! release and migrate refuse to run in secondary workspaces, warn when the
//! primary workspace is undeterminable, and stay silent without version
//! control.

mod common;

use common::TestResult;
use std::path::Path;
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

fn tool(dir: &Path, tool: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(tool)
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
}

fn tool_available(tool_name: &str) -> bool {
    Command::new(tool_name)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn run_tool(dir: &Path, tool_name: &str, args: &[&str]) -> TestResult {
    let output = tool(dir, tool_name, args)?;
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn trunk_commands_refused_in_secondary_git_worktree() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let secondary = holder.path().join("secondary");
    run_tool(
        main.path(),
        "git",
        &[
            "worktree",
            "add",
            secondary.to_str().ok_or("non-utf8 worktree path")?,
            "-b",
            "secondary",
        ],
    )?;

    let primary = std::fs::canonicalize(main.path())?;
    for args in [&["release", "0.1.0"][..], &["migrate"][..]] {
        let output = govctl(&secondary, args)?;
        assert!(
            !output.status.success(),
            "govctl {args:?} must fail in a secondary worktree"
        );
        let stderr = stderr(&output);
        assert!(stderr.contains("E0823"), "unexpected stderr: {stderr}");
        assert!(
            stderr.contains(&primary.display().to_string()),
            "diagnostic must name the primary workspace {primary:?}: {stderr}"
        );
    }

    // The refusal must leave governed state unchanged.
    assert!(!secondary.join("gov/releases.toml").exists());
    Ok(())
}

#[test]
fn trunk_commands_run_in_main_git_worktree_without_warning() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let date = common::today();
    let wi = common::first_work_id(&date);
    common::run_dynamic_commands(
        main.path(),
        &[
            common::work_new_active("Release work"),
            common::work_add_acceptance(&wi, "add: Something"),
            common::work_tick_acceptance_done(&wi, 0),
            common::work_move_done(&wi),
        ],
    )?;

    let release = govctl(main.path(), &["release", "0.1.0"])?;
    assert!(
        release.status.success(),
        "release failed in the main worktree: {}",
        stderr(&release)
    );
    let migrate = govctl(main.path(), &["migrate"])?;
    assert!(
        migrate.status.success(),
        "migrate failed in the main worktree: {}",
        stderr(&migrate)
    );
    for output in [&release, &migrate] {
        let stderr = stderr(output);
        assert!(
            !stderr.contains("warning"),
            "no trunk-scope warning expected in the main worktree: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn no_vcs_directory_runs_trunk_commands_silently() -> TestResult {
    let temp = common::init_project()?;
    let migrate = govctl(temp.path(), &["migrate"])?;
    assert!(
        migrate.status.success(),
        "migrate failed without version control: {}",
        stderr(&migrate)
    );
    let release = govctl(temp.path(), &["release", "0.1.0", "--dry-run"])?;
    let stderr = stderr(&release);
    assert!(
        !stderr.contains("warning"),
        "no trunk-scope warning expected without version control: {stderr}"
    );
    Ok(())
}

/// Create a non-colocated jj workspace around an existing govctl project.
fn init_jj_project() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = common::init_project()?;
    run_tool(
        temp.path(),
        "jj",
        &["--config", "git.colocate=false", "git", "init"],
    )?;
    Ok(temp)
}

/// Write the primary jj workspace name into the project's config.
fn configure_primary(dir: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let existing = std::fs::read_to_string(&config_path)?;
    std::fs::write(
        config_path,
        format!("{existing}\n[workspace]\nprimary = \"{name}\"\n"),
    )?;
    Ok(())
}

/// Commit the gov tree and add a named jj workspace beside the project,
/// returning its path.
fn add_jj_workspace(
    main: &Path,
    holder: &TempDir,
    name: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // Commit the gov tree so the added workspace checks it out.
    run_tool(main, "jj", &["describe", "-m", "init"])?;
    run_tool(main, "jj", &["new"])?;
    let workspace = holder.path().join(name);
    run_tool(
        main,
        "jj",
        &[
            "workspace",
            "add",
            "--name",
            name,
            workspace.to_str().ok_or("non-utf8 workspace path")?,
        ],
    )?;
    Ok(workspace)
}

#[test]
fn bare_repo_worktree_trunk_command_warns_and_proceeds() -> TestResult {
    if !tool_available("git") {
        return Ok(());
    }
    let main = init_git_project()?;
    let holder = TempDir::new()?;
    let bare = holder.path().join("bare.git");
    run_tool(
        main.path(),
        "git",
        &[
            "clone",
            "--bare",
            main.path().to_str().ok_or("non-utf8 repo path")?,
            bare.to_str().ok_or("non-utf8 bare path")?,
        ],
    )?;
    let worktree = holder.path().join("wt");
    run_tool(
        &bare,
        "git",
        &[
            "worktree",
            "add",
            worktree.to_str().ok_or("non-utf8 worktree path")?,
        ],
    )?;

    // [[RFC-0010:C-COMMAND-SCOPE]]: the parent of a bare repo's common dir
    // is not a working tree, so the primary workspace is undeterminable;
    // trunk commands must warn and proceed, never refuse while naming a
    // bogus primary path.
    let output = govctl(&worktree, &["migrate"])?;
    assert!(
        output.status.success(),
        "migrate must proceed when the primary workspace is undeterminable: {}",
        stderr(&output)
    );
    let stderr = stderr(&output);
    assert!(
        stderr.contains("W0114"),
        "expected enforcement-inactive warning: {stderr}"
    );
    assert!(
        !stderr.contains("E0823"),
        "must not refuse with a bogus primary: {stderr}"
    );
    Ok(())
}

#[test]
fn jj_unconfigured_default_workspace_runs_trunk_command_silently() -> TestResult {
    if !tool_available("jj") {
        return Ok(());
    }
    let temp = init_jj_project()?;
    // [[RFC-0010:C-COMMAND-SCOPE]]: an unconfigured jj repo treats its
    // initial `default` workspace as the primary.
    let output = govctl(temp.path(), &["migrate"])?;
    assert!(
        output.status.success(),
        "migrate failed in the default workspace: {}",
        stderr(&output)
    );
    let stderr = stderr(&output);
    assert!(
        !stderr.contains("warning"),
        "no trunk-scope warning expected in the default workspace: {stderr}"
    );
    Ok(())
}

#[test]
fn jj_unconfigured_added_workspace_is_refused() -> TestResult {
    if !tool_available("jj") {
        return Ok(());
    }
    let main = init_jj_project()?;
    let holder = TempDir::new()?;
    let secondary = add_jj_workspace(main.path(), &holder, "secondary")?;

    let primary = std::fs::canonicalize(main.path())?;
    let output = govctl(&secondary, &["migrate"])?;
    assert!(
        !output.status.success(),
        "migrate must fail in an added jj workspace without configuration"
    );
    let stderr = stderr(&output);
    assert!(stderr.contains("E0823"), "unexpected stderr: {stderr}");
    assert!(
        stderr.contains(&primary.display().to_string()),
        "diagnostic must name the default workspace {primary:?}: {stderr}"
    );
    Ok(())
}

#[test]
fn jj_undeterminable_primary_trunk_failure_still_warns() -> TestResult {
    if !tool_available("jj") {
        return Ok(());
    }
    let temp = init_jj_project()?;
    configure_primary(temp.path(), "nonexistent")?;
    // The primary workspace is undeterminable, so enforcement is inactive and
    // the release proceeds — then fails downstream with no unreleased work
    // items. [[RFC-0010:C-COMMAND-SCOPE]]: the enforcement-inactive warning
    // must be emitted even when the trunk command itself fails.
    let output = govctl(temp.path(), &["release", "0.1.0"])?;
    assert!(
        !output.status.success(),
        "release must fail with no unreleased work items"
    );
    let stderr = stderr(&output);
    assert!(
        stderr.contains("E0703"),
        "expected the downstream failure: {stderr}"
    );
    assert!(
        stderr.contains("W0114"),
        "the trunk-scope warning must survive the command failure: {stderr}"
    );
    Ok(())
}

#[test]
fn jj_configured_primary_runs_in_named_workspace_and_refuses_elsewhere() -> TestResult {
    if !tool_available("jj") {
        return Ok(());
    }
    let main = init_jj_project()?;
    // Configure before adding the workspace so both checkouts carry it.
    configure_primary(main.path(), "other")?;
    let holder = TempDir::new()?;
    let other = add_jj_workspace(main.path(), &holder, "other")?;

    let output = govctl(&other, &["migrate"])?;
    assert!(
        output.status.success(),
        "migrate failed in the configured primary workspace: {}",
        stderr(&output)
    );
    let named_stderr = stderr(&output);
    assert!(
        !named_stderr.contains("warning"),
        "no trunk-scope warning expected in the configured primary: {named_stderr}"
    );

    let primary = std::fs::canonicalize(&other)?;
    let output = govctl(main.path(), &["migrate"])?;
    assert!(
        !output.status.success(),
        "migrate must fail outside the configured primary workspace"
    );
    let stderr = stderr(&output);
    assert!(stderr.contains("E0823"), "unexpected stderr: {stderr}");
    assert!(
        stderr.contains(&primary.display().to_string()),
        "diagnostic must name the configured primary workspace: {stderr}"
    );
    Ok(())
}

#[test]
fn jj_configured_unknown_primary_warns_and_proceeds() -> TestResult {
    if !tool_available("jj") {
        return Ok(());
    }
    let temp = init_jj_project()?;
    configure_primary(temp.path(), "nonexistent")?;
    // The configured name is unknown to the repo, so the primary workspace
    // is undeterminable: warn and proceed, never refuse.
    let output = govctl(temp.path(), &["migrate"])?;
    assert!(
        output.status.success(),
        "migrate must proceed when the primary workspace is undeterminable: {}",
        stderr(&output)
    );
    let stderr = stderr(&output);
    assert!(
        stderr.contains("W0114"),
        "expected enforcement-inactive warning: {stderr}"
    );
    assert!(
        !stderr.contains("E0823"),
        "must not refuse while the primary is undeterminable: {stderr}"
    );
    Ok(())
}
