//! A dry-run of `work move done` reports the guard set and does not execute it.
//!
//! Implements [[RFC-0001:C-GATE-CONDITIONS]] and [[RFC-0002:C-LIFECYCLE-VERBS]].

mod common;

use common::{TestResult, git_init_isolated, init_project, run_commands, write_guard};
use std::fs;
use std::path::Path;
use std::process::Command;

const GUARD_ID: &str = "GUARD-TOUCH";

fn write_item(dir: &Path, id: &str, criteria_status: &str, verification: &str) -> TestResult {
    let content = format!(
        "[govctl]\nid = \"{id}\"\ntitle = \"Preview close\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Preview the done transition\"\n\n[[content.acceptance_criteria]]\ntext = \"criteria are complete\"\nstatus = \"{criteria_status}\"\ncategory = \"chore\"\n{verification}"
    );
    fs::write(dir.join(format!("gov/work/{id}.toml")), content)?;
    Ok(())
}

fn marker(dir: &Path) -> std::path::PathBuf {
    dir.join("ran-from-guard")
}

#[test]
fn dry_run_reports_the_set_and_a_real_move_still_runs_it() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let id = "WI-2026-01-01-001";
    write_guard(dir, GUARD_ID, "touch ran-from-guard")?;
    write_item(
        dir,
        id,
        "done",
        "\n[verification]\nrequired_guards = [\"GUARD-TOUCH\"]\n",
    )?;
    let before = fs::read(dir.join(format!("gov/work/{id}.toml")))?;

    let preview = run_commands(dir, &[&["--dry-run", "work", "move", id, "done"]])?;
    assert!(preview.contains("exit: 0"), "output: {preview}");
    assert!(preview.contains(id), "output: {preview}");
    assert!(preview.contains(GUARD_ID), "output: {preview}");
    assert!(preview.contains("Done plan"), "missing plan: {preview}");
    assert!(
        !preview.to_ascii_lowercase().contains("passed"),
        "claimed a pass: {preview}"
    );
    assert!(!marker(dir).exists(), "dry-run ran the guard");
    assert_eq!(before, fs::read(dir.join(format!("gov/work/{id}.toml")))?);

    let closed = run_commands(dir, &[&["work", "move", id, "done"]])?;
    assert!(closed.contains("exit: 0"), "output: {closed}");
    assert!(marker(dir).exists(), "the real move did not run the guard");
    let status = run_commands(dir, &[&["work", "get", id, "status"]])?;
    assert!(status.contains("done"), "output: {status}");
    Ok(())
}

#[test]
fn dry_run_states_an_empty_guard_set() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let id = "WI-2026-01-01-002";
    write_item(dir, id, "done", "")?;

    let preview = run_commands(dir, &[&["--dry-run", "work", "move", id, "done"]])?;
    assert!(preview.contains("exit: 0"), "output: {preview}");
    assert!(preview.contains(id), "output: {preview}");
    assert!(
        preview.contains("guards none"),
        "empty set was not stated: {preview}"
    );
    assert!(
        !preview.to_ascii_lowercase().contains("passed"),
        "claimed a pass: {preview}"
    );
    let status = run_commands(dir, &[&["work", "get", id, "status"]])?;
    assert!(status.contains("active"), "output: {status}");
    Ok(())
}

#[test]
fn dry_run_rejects_pending_criteria_without_a_preview() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let id = "WI-2026-01-01-003";
    write_guard(dir, GUARD_ID, "touch ran-from-guard")?;
    write_item(
        dir,
        id,
        "pending",
        "\n[verification]\nrequired_guards = [\"GUARD-TOUCH\"]\n",
    )?;

    let preview = run_commands(dir, &[&["--dry-run", "work", "move", id, "done"]])?;
    assert!(preview.contains("exit: 1"), "output: {preview}");
    assert!(preview.contains("E0407"), "output: {preview}");
    assert!(
        !preview.contains("Done plan"),
        "previewed a rejection: {preview}"
    );
    assert!(
        !preview.contains("Would write"),
        "wrote a preview: {preview}"
    );
    assert!(!marker(dir).exists(), "rejection ran the guard");
    let status = run_commands(dir, &[&["work", "get", id, "status"]])?;
    assert!(status.contains("active"), "output: {status}");
    Ok(())
}

#[test]
fn dry_run_rejects_an_unknown_guard_without_running_another() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let id = "WI-2026-01-01-004";
    write_guard(dir, GUARD_ID, "touch ran-from-guard")?;
    write_item(
        dir,
        id,
        "done",
        "\n[verification]\nrequired_guards = [\"GUARD-TOUCH\", \"GUARD-MISSING\"]\n",
    )?;

    let preview = run_commands(dir, &[&["--dry-run", "work", "move", id, "done"]])?;
    assert!(preview.contains("exit: 1"), "output: {preview}");
    assert!(
        preview.contains("E1002") || preview.contains("E1004"),
        "output: {preview}"
    );
    assert!(
        !preview.contains("Done plan"),
        "previewed a rejection: {preview}"
    );
    assert!(
        !preview.contains("Would write"),
        "wrote a preview: {preview}"
    );
    assert!(!marker(dir).exists(), "rejection ran the guard");

    let closed = run_commands(dir, &[&["work", "move", id, "done"]])?;
    assert!(closed.contains("exit: 1"), "output: {closed}");
    assert!(!marker(dir).exists(), "the real move ran a guard");
    let status = run_commands(dir, &[&["work", "get", id, "status"]])?;
    assert!(status.contains("active"), "output: {status}");
    Ok(())
}

#[test]
fn dry_run_reports_when_an_unrelated_guard_file_does_not_parse() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let id = "WI-2026-01-01-005";
    write_guard(dir, GUARD_ID, "touch ran-from-guard")?;
    fs::write(dir.join("gov/guard/guard-broken.toml"), "not = [\n")?;
    write_item(
        dir,
        id,
        "done",
        "\n[verification]\nrequired_guards = [\"GUARD-TOUCH\"]\n",
    )?;

    let preview = run_commands(dir, &[&["--dry-run", "work", "move", id, "done"]])?;
    assert!(preview.contains("exit: 0"), "output: {preview}");
    assert!(preview.contains("Done plan"), "output: {preview}");
    assert!(preview.contains(GUARD_ID), "output: {preview}");
    assert!(!marker(dir).exists(), "dry-run ran the guard");
    Ok(())
}

#[test]
fn dry_run_rejects_a_waiver_outside_the_required_set() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let id = "WI-2026-01-01-006";
    write_guard(dir, GUARD_ID, "touch ran-from-guard")?;
    write_guard(dir, "GUARD-OTHER", "true")?;
    write_item(
        dir,
        id,
        "done",
        "\n[verification]\nrequired_guards = [\"GUARD-TOUCH\"]\n\n[[verification.waivers]]\nguard = \"GUARD-OTHER\"\nreason = \"not required\"\n",
    )?;

    let preview = run_commands(dir, &[&["--dry-run", "work", "move", id, "done"]])?;
    assert!(preview.contains("exit: 1"), "output: {preview}");
    assert!(
        !preview.contains("Done plan"),
        "previewed a rejection: {preview}"
    );
    assert!(!marker(dir).exists(), "rejection ran the guard");
    Ok(())
}

#[test]
fn evidence_rejects_an_unknown_required_guard() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let id = "WI-2026-01-01-007";
    write_guard(dir, GUARD_ID, "touch ran-from-guard")?;
    write_item(
        dir,
        id,
        "done",
        "\n[verification]\nrequired_guards = [\"GUARD-TOUCH\", \"GUARD-MISSING\"]\n",
    )?;
    git_init_isolated(dir)?;
    let commit = Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()?;
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    let commit = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()?;
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()?;
    let revision = String::from_utf8_lossy(&head.stdout).trim().to_string();
    fs::write(
        dir.join("pass.toml"),
        format!(
            "work_id = \"{id}\"\nrevision = \"{revision}\"\nguards = [\"GUARD-MISSING\", \"GUARD-TOUCH\"]\n"
        ),
    )?;

    let closed = run_commands(
        dir,
        &[&["work", "move", id, "done", "--evidence", "pass.toml"]],
    )?;
    assert!(closed.contains("exit: 1"), "output: {closed}");
    assert!(!marker(dir).exists(), "evidence ran the guard");
    let status = run_commands(dir, &[&["work", "get", id, "status"]])?;
    assert!(status.contains("active"), "output: {status}");
    Ok(())
}
