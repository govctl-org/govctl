//! A pass record satisfies the guard portion of `done` for one checkout.
//!
//! Implements [[RFC-0001:C-PASS-RECORD]] and [[RFC-0002:C-PASS-RECORD-COMMAND]].

mod common;

use common::{TestResult, git_init_isolated, init_project, write_guard};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const WORK_ID: &str = "WI-2026-01-01-001";
const GUARD_ID: &str = "GUARD-TOUCH";

fn govctl(dir: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn git(dir: &Path, args: &[&str]) -> TestResult {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        text(&output.stderr)
    );
    Ok(())
}

fn head(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()?;
    assert!(output.status.success(), "rev-parse failed");
    Ok(text(&output.stdout).trim().to_string())
}

fn work_path(dir: &Path) -> PathBuf {
    dir.join("gov/work/2026-01-01-recorded.toml")
}

fn write_item(dir: &Path, guards: &[&str], criteria_status: &str) -> TestResult {
    let required = guards
        .iter()
        .map(|guard| format!("\"{guard}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let verification = if guards.is_empty() {
        String::new()
    } else {
        format!("\n[verification]\nrequired_guards = [{required}]\n")
    };
    let content = format!(
        "[govctl]\nid = \"{WORK_ID}\"\ntitle = \"Recorded item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Close from a pass record\"\n\n[[content.acceptance_criteria]]\ntext = \"criteria are complete\"\nstatus = \"{criteria_status}\"\ncategory = \"chore\"\n{verification}"
    );
    fs::write(work_path(dir), content)?;
    Ok(())
}

fn ignore_marker(dir: &Path) -> TestResult {
    let path = dir.join(".gitignore");
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    if !text.ends_with('\n') && !text.is_empty() {
        text.push('\n');
    }
    text.push_str("ignored-marker\n");
    fs::write(path, text)?;
    Ok(())
}

fn commit_project(dir: &Path) -> TestResult {
    git_init_isolated(dir)?;
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-q", "-m", "init"])?;
    Ok(())
}

fn marker(dir: &Path) -> PathBuf {
    dir.join("ignored-marker")
}

fn record_path(dir: &Path) -> PathBuf {
    dir.join("pass.toml")
}

struct Recorded {
    dir: tempfile::TempDir,
    revision: String,
}

fn recorded_project() -> Result<Recorded, Box<dyn std::error::Error>> {
    let dir = init_project()?;
    write_guard(dir.path(), GUARD_ID, "touch ignored-marker")?;
    write_item(dir.path(), &[GUARD_ID], "done")?;
    ignore_marker(dir.path())?;
    commit_project(dir.path())?;
    let revision = head(dir.path())?;
    Ok(Recorded { dir, revision })
}

#[test]
fn record_then_close_without_running_guards_again() -> TestResult {
    let recorded = recorded_project()?;
    let dir = recorded.dir.path();
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    let stderr = text(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(marker(dir).exists(), "guard did not run while recording");
    let record = fs::read_to_string(record_path(dir))?;
    assert!(
        record.contains(&format!("work_id = \"{WORK_ID}\"")),
        "{record}"
    );
    assert!(
        record.contains(&format!("revision = \"{}\"", recorded.revision)),
        "{record}"
    );
    assert!(record.contains(GUARD_ID), "{record}");

    fs::remove_file(marker(dir))?;
    let closed = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    let closed_err = text(&closed.stderr);
    assert!(closed.status.success(), "stderr: {closed_err}");
    assert!(!marker(dir).exists(), "evidence ran the guard");
    let status = govctl(dir, &["work", "get", WORK_ID, "status"])?;
    assert_eq!(text(&status.stdout).trim(), "done");
    Ok(())
}

#[test]
fn later_commit_matches_only_when_the_record_file_is_the_difference() -> TestResult {
    let recorded = recorded_project()?;
    let dir = recorded.dir.path();
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    git(dir, &["add", "pass.toml"])?;
    git(dir, &["commit", "-q", "-m", "record"])?;
    fs::remove_file(marker(dir))?;

    let closed = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    assert!(closed.status.success(), "stderr: {}", text(&closed.stderr));
    assert!(!marker(dir).exists(), "evidence ran the guard");

    let extra = recorded_project()?;
    let extra_dir = extra.dir.path();
    let output = govctl(
        extra_dir,
        &["verify", "--work", WORK_ID, "--record", "pass.toml"],
    )?;
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    fs::write(extra_dir.join("extra.txt"), "later\n")?;
    git(extra_dir, &["add", "pass.toml", "extra.txt"])?;
    git(extra_dir, &["commit", "-q", "-m", "extra"])?;
    fs::remove_file(marker(extra_dir))?;
    let before = fs::read(work_path(extra_dir))?;
    let rejected = govctl(
        extra_dir,
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    let stderr = text(&rejected.stderr);
    assert!(!rejected.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert!(
        !marker(extra_dir).exists(),
        "rejected evidence ran the guard"
    );
    assert_eq!(before, fs::read(work_path(extra_dir))?);
    Ok(())
}

#[test]
fn dirty_checkout_and_dirty_guard_reject_without_writing_the_record() -> TestResult {
    let recorded = recorded_project()?;
    let dir = recorded.dir.path();
    fs::write(record_path(dir), "sentinel\n")?;
    fs::write(dir.join("extra.txt"), "dirty\n")?;
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    let stderr = text(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert!(!marker(dir).exists(), "rejected record ran the guard");
    assert_eq!(fs::read(record_path(dir))?, b"sentinel\n");

    let clean = recorded_project()?;
    let clean_dir = clean.dir.path();
    write_guard(clean_dir, GUARD_ID, "touch dirty-from-guard")?;
    git(clean_dir, &["add", "-A"])?;
    git(clean_dir, &["commit", "-q", "-m", "dirty guard"])?;
    let output = govctl(
        clean_dir,
        &["verify", "--work", WORK_ID, "--record", "pass.toml"],
    )?;
    let stderr = text(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert!(
        clean_dir.join("dirty-from-guard").exists(),
        "guard did not run"
    );
    assert!(
        !record_path(clean_dir).exists(),
        "record written after a dirty guard"
    );
    Ok(())
}

#[test]
fn missing_vcs_and_unreadable_metadata_reject_without_running_guards() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    write_guard(dir, GUARD_ID, "touch ignored-marker")?;
    write_item(dir, &[GUARD_ID], "done")?;
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    let stderr = text(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert!(!stderr.contains("W0115"), "stderr: {stderr}");
    assert!(!marker(dir).exists());
    assert!(!record_path(dir).exists());

    fs::write(dir.join(".git"), "not a git directory\n")?;
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    let stderr = text(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("W0115"), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert!(!marker(dir).exists());
    assert!(!record_path(dir).exists());
    Ok(())
}

#[test]
fn empty_guard_set_rejects_a_record_and_plain_done_still_succeeds() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    write_item(dir, &[], "done")?;
    commit_project(dir)?;
    let recorded = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    let stderr = text(&recorded.stderr);
    assert!(!recorded.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert!(!record_path(dir).exists());

    fs::write(record_path(dir), "work_id = \"x\"\n")?;
    let before = fs::read(work_path(dir))?;
    let evidence = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    let stderr = text(&evidence.stderr);
    assert!(!evidence.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert_eq!(before, fs::read(work_path(dir))?);

    let closed = govctl(dir, &["work", "move", WORK_ID, "done"])?;
    assert!(closed.status.success(), "stderr: {}", text(&closed.stderr));
    let status = govctl(dir, &["work", "get", WORK_ID, "status"])?;
    assert_eq!(text(&status.stdout).trim(), "done");
    Ok(())
}

#[test]
fn evidence_still_requires_criteria_and_rejects_other_statuses() -> TestResult {
    let recorded = recorded_project()?;
    let dir = recorded.dir.path();
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    fs::remove_file(marker(dir))?;

    let pending = init_project()?;
    write_guard(pending.path(), GUARD_ID, "touch ignored-marker")?;
    write_item(pending.path(), &[GUARD_ID], "pending")?;
    ignore_marker(pending.path())?;
    commit_project(pending.path())?;
    fs::copy(record_path(dir), record_path(pending.path()))?;
    // The copied record names the other checkout's revision, so rewrite is not
    // required: this case is rejected for pending criteria before guards, and
    // the work item stays unchanged either way.
    let before = fs::read(work_path(pending.path()))?;
    let rejected = govctl(
        pending.path(),
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    let stderr = text(&rejected.stderr);
    assert!(!rejected.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0407"), "stderr: {stderr}");
    assert!(!marker(pending.path()).exists());
    assert_eq!(before, fs::read(work_path(pending.path()))?);

    let evidence = fs::read(record_path(dir))?;
    let other = govctl(
        dir,
        &[
            "work",
            "move",
            WORK_ID,
            "cancelled",
            "--evidence",
            "pass.toml",
        ],
    )?;
    let stderr = text(&other.stderr);
    assert!(!other.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0822"), "stderr: {stderr}");
    assert_eq!(evidence, fs::read(record_path(dir))?);
    let status = govctl(dir, &["work", "get", WORK_ID, "status"])?;
    assert_eq!(text(&status.stdout).trim(), "active");
    Ok(())
}

#[test]
fn dry_run_reports_the_plan_without_claiming_the_guards_passed() -> TestResult {
    let recorded = recorded_project()?;
    let dir = recorded.dir.path();
    let output = govctl(
        dir,
        &[
            "--dry-run",
            "verify",
            "--work",
            WORK_ID,
            "--record",
            "pass.toml",
        ],
    )?;
    let stderr = text(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains(WORK_ID), "stderr: {stderr}");
    assert!(stderr.contains(&recorded.revision), "stderr: {stderr}");
    assert!(stderr.contains(GUARD_ID), "stderr: {stderr}");
    let lowered = stderr.to_ascii_lowercase();
    assert!(!lowered.contains("passed"), "stderr: {stderr}");
    assert!(!marker(dir).exists(), "dry-run ran the guard");
    assert!(!record_path(dir).exists(), "dry-run wrote the record");
    Ok(())
}

#[test]
fn bad_records_are_rejected_without_running_guards() -> TestResult {
    let recorded = recorded_project()?;
    let dir = recorded.dir.path();
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    fs::remove_file(marker(dir))?;
    let good = fs::read_to_string(record_path(dir))?;

    for body in [
        format!("{good}extra = true\n"),
        format!(
            "work_id = \"{WORK_ID}\"\nrevision = \"{}\"\nguards = [\"{GUARD_ID}\", \"{GUARD_ID}\"]\n",
            recorded.revision
        ),
        format!(
            "work_id = \"WI-2026-01-01-999\"\nrevision = \"{}\"\nguards = [\"{GUARD_ID}\"]\n",
            recorded.revision
        ),
    ] {
        fs::write(record_path(dir), &body)?;
        let before = fs::read(work_path(dir))?;
        let rejected = govctl(
            dir,
            &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
        )?;
        let stderr = text(&rejected.stderr);
        assert!(!rejected.status.success(), "body: {body}\nstderr: {stderr}");
        assert!(stderr.contains("E0820"), "stderr: {stderr}");
        assert!(!marker(dir).exists(), "bad record ran the guard");
        assert_eq!(before, fs::read(work_path(dir))?);
        assert_eq!(fs::read_to_string(record_path(dir))?, body);
    }
    Ok(())
}

#[test]
fn record_flags_are_rejected_when_the_command_shape_is_wrong() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let missing_work = govctl(dir, &["verify", "--record", "pass.toml"])?;
    assert!(!missing_work.status.success());
    let with_guard = govctl(
        dir,
        &[
            "verify",
            "--work",
            WORK_ID,
            "--record",
            "pass.toml",
            GUARD_ID,
        ],
    )?;
    assert!(!with_guard.status.success());
    Ok(())
}

#[test]
fn a_record_that_counts_a_waived_guard_as_passed_is_rejected() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    write_guard(dir, GUARD_ID, "touch ignored-marker")?;
    write_guard(dir, "GUARD-OTHER", "true")?;
    write_item(dir, &[GUARD_ID, "GUARD-OTHER"], "done")?;
    let mut item = fs::read_to_string(work_path(dir))?;
    item.push_str(
        "\n[[verification.waivers]]\nguard = \"GUARD-OTHER\"\nreason = \"does not apply\"\n",
    );
    fs::write(work_path(dir), item)?;
    ignore_marker(dir)?;
    commit_project(dir)?;
    let revision = head(dir)?;
    let before = fs::read(work_path(dir))?;

    let both = format!(
        "work_id = \"{WORK_ID}\"\nrevision = \"{revision}\"\nguards = [\"{GUARD_ID}\", \"GUARD-OTHER\"]\n"
    );
    fs::write(record_path(dir), &both)?;
    let rejected = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    let stderr = text(&rejected.stderr);
    assert!(!rejected.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "stderr: {stderr}");
    assert!(!marker(dir).exists(), "mismatched record ran the guard");
    assert_eq!(before, fs::read(work_path(dir))?);
    assert_eq!(fs::read_to_string(record_path(dir))?, both);

    fs::write(
        record_path(dir),
        format!("work_id = \"{WORK_ID}\"\nrevision = \"{revision}\"\nguards = [\"{GUARD_ID}\"]\n"),
    )?;
    let closed = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    assert!(closed.status.success(), "stderr: {}", text(&closed.stderr));
    assert!(!marker(dir).exists(), "matching evidence ran the guard");
    Ok(())
}

#[test]
fn unicode_record_path_closes_when_it_is_the_only_extra() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    write_guard(dir, GUARD_ID, "touch ignored-marker")?;
    write_item(dir, &[GUARD_ID], "done")?;
    ignore_marker(dir)?;
    commit_project(dir)?;
    git(dir, &["config", "core.quotepath", "true"])?;
    fs::create_dir(dir.join("notes"))?;
    let record = "notes/ünicode.toml";

    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", record])?;
    let stderr = text(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(dir.join(record).is_file(), "record was not written");
    assert!(marker(dir).exists(), "guard did not run while recording");

    fs::remove_file(marker(dir))?;
    let closed = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", record],
    )?;
    let closed_err = text(&closed.stderr);
    assert!(closed.status.success(), "stderr: {closed_err}");
    assert!(!marker(dir).exists(), "evidence ran the guard");
    let status = govctl(dir, &["work", "get", WORK_ID, "status"])?;
    assert_eq!(text(&status.stdout).trim(), "done");
    Ok(())
}

#[test]
fn lookalike_names_do_not_count_as_the_pass_record() -> TestResult {
    let recorded = recorded_project()?;
    let dir = recorded.dir.path();
    git(dir, &["config", "core.quotepath", "true"])?;
    fs::create_dir(dir.join("notes"))?;
    let record = "notes/pass.toml";
    let lookalikes = [
        dir.join("notes\\pass.toml"),
        dir.join("notes").join("pass.toml\t"),
        dir.join("notes").join("pass.toml\r"),
    ];
    for extra in &lookalikes {
        fs::write(extra, "not the record\n")?;
        let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", record])?;
        let stderr = text(&output.stderr);
        assert!(
            !output.status.success(),
            "accepted {}: {stderr}",
            extra.display()
        );
        assert!(stderr.contains("E0820"), "{stderr}");
        assert!(
            !dir.join(record).exists(),
            "wrote the record while {} was present",
            extra.display()
        );
        assert!(
            !marker(dir).exists(),
            "ran the guard while {} was present",
            extra.display()
        );
        let status = govctl(dir, &["work", "get", WORK_ID, "status"])?;
        assert_eq!(text(&status.stdout).trim(), "active");
        fs::remove_file(extra)?;
    }

    let extra = dir.join("notes\\pass.toml");
    fs::write(&extra, "not the record\n")?;
    let record_file = dir.join(record);
    let record_text = format!(
        "work_id = \"{WORK_ID}\"\nrevision = \"{}\"\nguards = [\"{GUARD_ID}\"]\n",
        recorded.revision
    );
    fs::write(&record_file, &record_text)?;
    let closed = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", record],
    )?;
    let stderr = text(&closed.stderr);
    assert!(!closed.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0820"), "{stderr}");
    assert_eq!(fs::read_to_string(&record_file)?, record_text);
    assert!(!marker(dir).exists(), "evidence ran the guard");
    let status = govctl(dir, &["work", "get", WORK_ID, "status"])?;
    assert_eq!(text(&status.stdout).trim(), "active");
    Ok(())
}

#[test]
fn recording_a_missing_work_item_does_not_create_a_record() -> TestResult {
    let temp = init_project()?;
    let output = govctl(
        temp.path(),
        &[
            "verify",
            "--work",
            "WI-2026-01-01-999",
            "--record",
            "pass.toml",
        ],
    )?;
    let stderr = text(&output.stderr);
    assert!(!output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0402"), "stderr: {stderr}");
    assert!(!record_path(temp.path()).exists());
    Ok(())
}

#[test]
fn jj_record_stores_the_shared_commit_id() -> TestResult {
    if Command::new("jj")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_none()
    {
        return Ok(());
    }
    let temp = init_project()?;
    let dir = temp.path();
    write_guard(dir, GUARD_ID, "touch ignored-marker")?;
    write_item(dir, &[GUARD_ID], "done")?;
    ignore_marker(dir)?;
    let init = Command::new("jj")
        .args(["--config", "git.colocate=false", "git", "init"])
        .current_dir(dir)
        .output()?;
    assert!(init.status.success(), "jj init: {}", text(&init.stderr));
    // Snapshot the working copy so the sampled commit already contains the files.
    let snap = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .current_dir(dir)
        .output()?;
    assert!(snap.status.success(), "jj log: {}", text(&snap.stderr));
    let revision = text(&snap.stdout).trim().to_string();
    let output = govctl(dir, &["verify", "--work", WORK_ID, "--record", "pass.toml"])?;
    let stderr = text(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let record = fs::read_to_string(record_path(dir))?;
    assert!(
        record.contains(&format!("revision = \"{revision}\"")),
        "record:\n{record}\nexpected {revision}"
    );
    fs::remove_file(marker(dir))?;
    let closed = govctl(
        dir,
        &["work", "move", WORK_ID, "done", "--evidence", "pass.toml"],
    )?;
    let stderr = text(&closed.stderr);
    assert!(closed.status.success(), "stderr: {stderr}");
    assert!(!marker(dir).exists(), "jj evidence ran the guard");
    Ok(())
}
