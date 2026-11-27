//! Work Item owners and `work list --owner`.
//!
//! Implements [[RFC-0000:C-WORK-DEF]] and [[RFC-0002:C-WORK-OWNERS]].

mod common;

use common::{TestResult, init_project, run_commands};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const ADA: &str = "WI-2026-01-01-001";
const GRACE_ONLY: &str = "WI-2026-01-01-002";
const QUEUED: &str = "WI-2026-01-01-003";

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

fn write_item(dir: &Path, id: &str, status: &str, owners: &str, tags: &str) -> TestResult {
    let owners_line = if owners.is_empty() {
        String::new()
    } else {
        format!("owners = [{owners}]\n")
    };
    let content = format!(
        "[govctl]\nid = \"{id}\"\ntitle = \"{id}\"\nstatus = \"{status}\"\ncreated = \"2026-01-01\"\n{owners_line}tags = [{tags}]\n\n[content]\ndescription = \"Owned work\"\n"
    );
    fs::write(dir.join(format!("gov/work/{id}.toml")), content)?;
    Ok(())
}

fn project() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp = init_project()?;
    let dir = temp.path().to_path_buf();
    write_item(&dir, ADA, "active", "\"ada\", \"grace\"", "\"cli\"")?;
    write_item(&dir, GRACE_ONLY, "active", "\"ada\"", "\"cli\"")?;
    write_item(&dir, QUEUED, "queue", "\"ada\", \"grace\"", "\"cli\"")?;
    write_item(&dir, "WI-2026-01-01-004", "active", "", "\"cli\"")?;
    Ok((temp, dir))
}

#[test]
fn list_requires_every_owner_and_combines_with_status_and_tag() -> TestResult {
    let (temp, dir) = project()?;
    let _ = temp;
    let both = run_commands(
        &dir,
        &[&[
            "work", "list", "--owner", "ada", "--owner", "grace", "-o", "json",
        ]],
    )?;
    assert!(both.contains("exit: 0"), "output: {both}");
    assert!(both.contains(ADA), "output: {both}");
    assert!(both.contains(QUEUED), "output: {both}");
    assert!(!both.contains(GRACE_ONLY), "output: {both}");

    let narrowed = run_commands(
        &dir,
        &[&[
            "work", "list", "active", "--owner", " ada ", "--owner", "grace", "--tag", "cli", "-o",
            "json",
        ]],
    )?;
    assert!(narrowed.contains("exit: 0"), "output: {narrowed}");
    assert!(narrowed.contains(ADA), "output: {narrowed}");
    assert!(!narrowed.contains(QUEUED), "output: {narrowed}");
    assert!(!narrowed.contains(GRACE_ONLY), "output: {narrowed}");
    Ok(())
}

#[test]
fn empty_owner_filter_is_rejected_and_no_match_is_empty_success() -> TestResult {
    let (temp, dir) = project()?;
    let _ = temp;
    let empty = govctl(&dir, &["work", "list", "--owner", "   "])?;
    let stderr = text(&empty.stderr);
    assert!(!empty.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("E0805"), "stderr: {stderr}");

    let none = run_commands(
        &dir,
        &[&["work", "list", "--owner", "nobody", "-o", "json"]],
    )?;
    assert!(none.contains("exit: 0"), "output: {none}");
    assert!(none.contains("[]"), "output: {none}");
    assert!(!none.contains(ADA), "output: {none}");

    let rfc = govctl(&dir, &["rfc", "list", "--owner", "ada"])?;
    let rfc_err = text(&rfc.stderr);
    assert!(!rfc.status.success(), "stderr: {rfc_err}");
    assert!(rfc_err.contains("unexpected argument"), "stderr: {rfc_err}");
    Ok(())
}

#[test]
fn owner_edits_trim_and_reject_empty_or_duplicate_values() -> TestResult {
    let (temp, dir) = project()?;
    let _ = temp;
    let path = dir.join(format!("gov/work/{ADA}.toml"));
    let before = fs::read(&path)?;

    let empty = run_commands(&dir, &[&["work", "edit", ADA, "owners", "--add", "   "]])?;
    assert!(empty.contains("E0805"), "output: {empty}");
    assert!(empty.contains("exit: 1"), "output: {empty}");
    assert_eq!(before, fs::read(&path)?);

    let duplicate = run_commands(
        &dir,
        &[&["work", "edit", ADA, "owners", "--add", "  ada  "]],
    )?;
    assert!(duplicate.contains("E0820"), "output: {duplicate}");
    assert!(duplicate.contains("exit: 1"), "output: {duplicate}");
    assert_eq!(before, fs::read(&path)?);

    let other_index = run_commands(
        &dir,
        &[&["work", "edit", ADA, "owners[0]", "--set", "grace"]],
    )?;
    assert!(other_index.contains("E0820"), "output: {other_index}");
    assert_eq!(before, fs::read(&path)?);

    let same_index = run_commands(
        &dir,
        &[&["work", "edit", ADA, "owners[0]", "--set", "  ada  "]],
    )?;
    assert!(same_index.contains("exit: 0"), "output: {same_index}");
    assert_eq!(before, fs::read(&path)?);

    let added = run_commands(
        &dir,
        &[&["work", "edit", ADA, "owners", "--add", "  linus  "]],
    )?;
    assert!(added.contains("exit: 0"), "output: {added}");
    let stored = run_commands(&dir, &[&["work", "get", ADA, "owners", "-o", "plain"]])?;
    assert!(stored.contains("linus"), "output: {stored}");
    assert!(!stored.contains("  linus"), "output: {stored}");

    let omitted = run_commands(
        &dir,
        &[&["work", "get", "WI-2026-01-01-004", "owners", "-o", "json"]],
    )?;
    assert!(omitted.contains("exit: 0"), "output: {omitted}");
    assert!(omitted.contains("[]"), "output: {omitted}");
    assert!(!omitted.contains("null"), "output: {omitted}");
    Ok(())
}

#[test]
fn check_rejects_owners_with_surrounding_whitespace() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let path = dir.join("gov/work/WI-2026-01-01-010.toml");

    for owners in ["\" \"", "\" ada\"", "\"ada \""] {
        fs::write(
            &path,
            format!(
                "[govctl]\nid = \"WI-2026-01-01-010\"\ntitle = \"Padded owner\"\nstatus = \"queue\"\ncreated = \"2026-01-01\"\nowners = [{owners}]\n\n[content]\ndescription = \"Stored owner must already be trimmed\"\n"
            ),
        )?;
        let output = run_commands(dir, &[&["check"]])?;
        assert!(output.contains("E0401"), "owners {owners}: {output}");
        assert!(output.contains("exit: 1"), "owners {owners}: {output}");
    }

    fs::write(
        &path,
        "[govctl]\nid = \"WI-2026-01-01-010\"\ntitle = \"Padded owner\"\nstatus = \"queue\"\ncreated = \"2026-01-01\"\nowners = [\"Ada Lovelace\"]\n\n[content]\ndescription = \"Stored owner must already be trimmed\"\n",
    )?;
    let output = run_commands(dir, &[&["check"]])?;
    assert!(output.contains("exit: 0"), "output: {output}");
    Ok(())
}
