//! Updates preserve Work Item text the command does not change.
//!
//! Implements [[RFC-0002:C-WORK-FILE-STABILITY]].

mod common;

use common::{TestResult, init_project, run_commands, write_guard};
use std::fs;
use std::path::{Path, PathBuf};

const WORK_ID: &str = "WI-2026-01-01-001";
const HEADER: &str = "# header stays";
const DESCRIPTION_NOTE: &str = "# description note stays";

fn work_path(dir: &Path) -> PathBuf {
    dir.join("gov/work/2026-01-01-stable.toml")
}

fn write_item(dir: &Path) -> TestResult {
    let content = format!(
        "{HEADER}\n\n[govctl]\nid = \"{WORK_ID}\"\ntitle = \"Old title\"\nstatus = \"queue\"\ncreated = \"2026-01-01\"\nowners = [\"ada\"]\n\n{DESCRIPTION_NOTE}\n[content]\ndescription = \"Body stays\"\nnotes = [\"keep this note\"]\n\n[verification]\nrequired_guards = [\"GUARD-KEPT\"]\n"
    );
    fs::write(work_path(dir), content)?;
    Ok(())
}

fn assert_unchanged_text(dir: &Path) -> TestResult {
    let text = fs::read_to_string(work_path(dir))?;
    assert!(text.contains(HEADER), "lost header:\n{text}");
    assert!(
        text.contains(DESCRIPTION_NOTE),
        "lost description comment:\n{text}"
    );
    assert!(
        text.contains("description = \"Body stays\""),
        "description changed:\n{text}"
    );
    assert!(
        text.contains("notes = [\"keep this note\"]"),
        "notes changed:\n{text}"
    );
    Ok(())
}

#[test]
fn edit_move_and_waive_preserve_unchanged_text() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    write_guard(dir, "GUARD-KEPT", "true")?;
    write_item(dir)?;

    let edited = run_commands(
        dir,
        &[&["work", "edit", WORK_ID, "title", "--set", "New title"]],
    )?;
    assert!(edited.contains("exit: 0"), "output: {edited}");
    let text = fs::read_to_string(work_path(dir))?;
    assert!(text.contains("title = \"New title\""), "title: {text}");
    assert_unchanged_text(dir)?;

    let moved = run_commands(dir, &[&["work", "move", WORK_ID, "active"]])?;
    assert!(moved.contains("exit: 0"), "output: {moved}");
    let text = fs::read_to_string(work_path(dir))?;
    assert!(text.contains("status = \"active\""), "status: {text}");
    assert_unchanged_text(dir)?;

    let waived = run_commands(
        dir,
        &[&[
            "work",
            "waive",
            WORK_ID,
            "GUARD-KEPT",
            "--reason",
            "Not exercised here",
        ]],
    )?;
    assert!(waived.contains("exit: 0"), "output: {waived}");
    let text = fs::read_to_string(work_path(dir))?;
    assert!(
        text.contains("reason = \"Not exercised here\""),
        "waiver missing:\n{text}"
    );
    assert_unchanged_text(dir)?;
    Ok(())
}

#[test]
fn title_edit_keeps_text_only_acceptance_criteria() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    write_guard(dir, "GUARD-KEPT", "true")?;
    let content = format!(
        "{HEADER}\n\n[govctl]\nid = \"{WORK_ID}\"\ntitle = \"Old title\"\nstatus = \"queue\"\ncreated = \"2026-01-01\"\n\n{DESCRIPTION_NOTE}\n[content]\ndescription = \"Body stays\"\n\n[[content.acceptance_criteria]]\ntext = \"only text\"\n\n[[content.acceptance_criteria]]\ntext = \"explicit defaults\"\nstatus = \"pending\"\ncategory = \"added\"\n\n[verification]\nrequired_guards = [\"GUARD-KEPT\"]\n"
    );
    fs::write(work_path(dir), content)?;

    let edited = run_commands(
        dir,
        &[&["work", "edit", WORK_ID, "title", "--set", "New title"]],
    )?;
    assert!(edited.contains("exit: 0"), "output: {edited}");
    assert_criteria_text_unchanged(dir)?;

    let moved = run_commands(dir, &[&["work", "move", WORK_ID, "active"]])?;
    assert!(moved.contains("exit: 0"), "output: {moved}");
    assert_criteria_text_unchanged(dir)?;

    let waived = run_commands(
        dir,
        &[&[
            "work",
            "waive",
            WORK_ID,
            "GUARD-KEPT",
            "--reason",
            "Not exercised here",
        ]],
    )?;
    assert!(waived.contains("exit: 0"), "output: {waived}");
    assert_criteria_text_unchanged(dir)?;
    Ok(())
}

fn assert_criteria_text_unchanged(dir: &Path) -> TestResult {
    let text = fs::read_to_string(work_path(dir))?;
    assert!(
        text.contains("[[content.acceptance_criteria]]\ntext = \"only text\"\n"),
        "text-only criterion was rewritten:\n{text}"
    );
    assert!(
        text.contains("text = \"explicit defaults\"\nstatus = \"pending\"\ncategory = \"added\""),
        "explicit defaults were rewritten:\n{text}"
    );
    let criteria = text
        .split("[[content.acceptance_criteria]]")
        .nth(1)
        .unwrap_or("");
    let only = criteria.split("explicit defaults").next().unwrap_or("");
    assert!(
        !only.contains("status = "),
        "omitted status was inserted:\n{text}"
    );
    assert!(
        !only.contains("category = "),
        "omitted category was inserted:\n{text}"
    );
    Ok(())
}

#[test]
fn title_edit_keeps_an_unchanged_inline_sibling() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    let content = format!(
        "{HEADER}\n\ngovctl = {{ id = \"{WORK_ID}\", title = \"Old title\", status = \"queue\", created = \"2026-01-01\", owners = [ \"ada\" ] }}\n\n[content]\ndescription = \"Body stays\"\n"
    );
    fs::write(work_path(dir), content)?;

    let edited = run_commands(
        dir,
        &[&["work", "edit", WORK_ID, "title", "--set", "New title"]],
    )?;
    assert!(edited.contains("exit: 0"), "output: {edited}");
    let text = fs::read_to_string(work_path(dir))?;
    assert!(
        text.contains("title = \"New title\"") || text.contains("title = 'New title'"),
        "title was not updated:\n{text}"
    );
    assert!(
        text.contains("owners = [ \"ada\" ]"),
        "inline sibling was rewritten:\n{text}"
    );
    assert!(text.contains(HEADER), "lost header:\n{text}");
    Ok(())
}

#[test]
fn same_index_owner_set_does_not_rewrite_the_file() -> TestResult {
    let temp = init_project()?;
    let dir = temp.path();
    write_item(dir)?;
    let before = fs::read(work_path(dir))?;
    let output = run_commands(
        dir,
        &[&["work", "edit", WORK_ID, "owners[0]", "--set", "  ada  "]],
    )?;
    assert!(output.contains("exit: 0"), "output: {output}");
    assert_eq!(before, fs::read(work_path(dir))?);
    Ok(())
}
