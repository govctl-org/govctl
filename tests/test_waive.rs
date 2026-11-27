//! `govctl work waive` records one Work Item verification waiver.
//!
//! Implements [[RFC-0002:C-WORK-WAIVE]].

mod common;

use common::{
    TestResult, append_verification_config, init_project, run_commands, temp_dir_with_date,
    write_canonical_guarded_work_item, write_guard,
};
use std::fs;
use std::path::{Path, PathBuf};

const WORK_ID: &str = "WI-2026-01-01-001";
const KEPT_GUARD: &str = "GUARD-KEPT";
const NEW_GUARD: &str = "GUARD-NEW";

fn work_path(dir: &Path) -> PathBuf {
    dir.join("gov/work/2026-01-01-guarded-item.toml")
}

fn command_body(output: &str) -> String {
    output
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with("exit:"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn field(dir: &Path, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = run_commands(dir, &[&["work", "get", WORK_ID, name]])?;
    Ok(command_body(&output))
}

fn write_item(
    dir: &Path,
    required: &[&str],
    waiver: Option<(&str, &str)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let required_guards = required
        .iter()
        .map(|guard_id| format!("\"{guard_id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let waiver = waiver
        .map(|(guard_id, reason)| {
            format!("\n[[verification.waivers]]\nguard = \"{guard_id}\"\nreason = \"{reason}\"\n")
        })
        .unwrap_or_default();
    let content = format!(
        "[govctl]\nid = \"{WORK_ID}\"\ntitle = \"Guarded Item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\nrefs = [\"RFC-0002:C-WORK-WAIVE\"]\ndepends_on = [\"WI-2026-01-01-002\"]\ntags = [\"cli\"]\n\n[content]\ndescription = \"Guarded work item\"\nnotes = [\"keep this note\"]\n\n[[content.acceptance_criteria]]\ntext = \"done criteria\"\nstatus = \"done\"\ncategory = \"chore\"\n\n[verification]\nrequired_guards = [{required_guards}]{waiver}"
    );
    fs::write(work_path(dir), content)?;
    Ok(())
}

fn project_with_guards(
    guard_ids: &[&str],
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp_dir = init_project()?;
    for guard_id in guard_ids {
        write_guard(temp_dir.path(), guard_id, "true")?;
    }
    Ok(temp_dir)
}

fn assert_rejected(
    dir: &Path,
    args: &[&str],
    code: &str,
    guard_id: &str,
    needle: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = fs::read(work_path(dir))?;
    let output = run_commands(dir, &[args])?;
    let after = fs::read(work_path(dir))?;
    let diagnostic = command_body(&output);
    assert_eq!(before, after, "work item bytes changed:\n{output}");
    assert!(diagnostic.contains(code), "output: {output}");
    assert!(diagnostic.contains(WORK_ID), "output: {output}");
    assert!(diagnostic.contains(guard_id), "output: {output}");
    assert!(diagnostic.contains(needle), "output: {output}");
    assert!(output.contains("exit: 1"), "output: {output}");
    assert!(!diagnostic.contains("Added '"), "output: {output}");
    Ok(())
}

#[test]
fn waive_appends_one_trimmed_waiver_and_leaves_other_fields() -> TestResult {
    let temp_dir = project_with_guards(&[KEPT_GUARD, NEW_GUARD])?;
    let dir = temp_dir.path();
    write_item(
        dir,
        &[KEPT_GUARD, NEW_GUARD],
        Some((KEPT_GUARD, "keep this reason")),
    )?;

    let title = field(dir, "title")?;
    let description = field(dir, "description")?;
    let status = field(dir, "status")?;
    let created = field(dir, "created")?;
    let started = field(dir, "started")?;
    let refs = field(dir, "refs")?;
    let depends_on = field(dir, "depends_on")?;
    let tags = field(dir, "tags")?;
    let notes = field(dir, "notes")?;
    let required = field(dir, "verification.required_guards")?;
    let criterion = field(dir, "acceptance_criteria[0].text")?;
    let category = field(dir, "acceptance_criteria[0].category")?;
    let kept_guard = field(dir, "verification.waivers[0].guard")?;
    let kept_reason = field(dir, "verification.waivers[0].reason")?;

    let output = run_commands(
        dir,
        &[&[
            "work",
            "waive",
            WORK_ID,
            NEW_GUARD,
            "--reason",
            "  because it does not apply  ",
        ]],
    )?;

    assert!(
        output.contains(&format!(
            "Added '{NEW_GUARD}' to {WORK_ID}.verification.waivers"
        )),
        "output: {output}"
    );
    assert!(output.contains("exit: 0"), "output: {output}");
    assert_eq!(field(dir, "title")?, title);
    assert_eq!(field(dir, "description")?, description);
    assert_eq!(field(dir, "status")?, status);
    assert_eq!(field(dir, "created")?, created);
    assert_eq!(field(dir, "started")?, started);
    assert_eq!(field(dir, "refs")?, refs);
    assert_eq!(field(dir, "depends_on")?, depends_on);
    assert_eq!(field(dir, "tags")?, tags);
    assert_eq!(field(dir, "notes")?, notes);
    assert_eq!(field(dir, "verification.required_guards")?, required);
    assert_eq!(field(dir, "acceptance_criteria[0].text")?, criterion);
    assert_eq!(field(dir, "acceptance_criteria[0].category")?, category);
    assert_eq!(field(dir, "verification.waivers[0].guard")?, kept_guard);
    assert_eq!(field(dir, "verification.waivers[0].reason")?, kept_reason);
    assert_eq!(kept_guard, KEPT_GUARD);
    assert_eq!(kept_reason, "keep this reason");
    assert_eq!(field(dir, "verification.waivers[1].guard")?, NEW_GUARD);
    assert_eq!(
        field(dir, "verification.waivers[1].reason")?,
        "because it does not apply"
    );
    let waivers = field(dir, "verification.waivers")?;
    assert_eq!(waivers.lines().count(), 2, "waivers: {waivers}");
    Ok(())
}

#[test]
fn waive_rejects_duplicate_unknown_empty_and_unrequired_guards() -> TestResult {
    let temp_dir = project_with_guards(&[NEW_GUARD, "GUARD-OTHER"])?;
    let dir = temp_dir.path();
    write_canonical_guarded_work_item(dir, WORK_ID, NEW_GUARD, Some("already waived"))?;

    assert_rejected(
        dir,
        &["work", "waive", WORK_ID, NEW_GUARD, "--reason", "again"],
        "error[E0401]",
        NEW_GUARD,
        "Duplicate",
    )?;
    assert_rejected(
        dir,
        &[
            "work",
            "waive",
            WORK_ID,
            "GUARD-MISSING",
            "--reason",
            "missing guard",
        ],
        "error[E1002]",
        "GUARD-MISSING",
        "unknown",
    )?;
    assert_rejected(
        dir,
        &["work", "waive", WORK_ID, NEW_GUARD, "--reason", "   "],
        "error[E0401]",
        NEW_GUARD,
        "must not be empty",
    )?;
    assert_rejected(
        dir,
        &["work", "waive", WORK_ID, NEW_GUARD],
        "error[E0401]",
        NEW_GUARD,
        "must not be empty",
    )?;
    assert_rejected(
        dir,
        &[
            "work",
            "waive",
            WORK_ID,
            "GUARD-OTHER",
            "--reason",
            "not required here",
        ],
        "error[E0401]",
        "GUARD-OTHER",
        "not required",
    )?;
    Ok(())
}

#[test]
fn waive_rejects_a_guard_id_that_contains_separator_text() -> TestResult {
    let temp_dir = project_with_guards(&[NEW_GUARD])?;
    let dir = temp_dir.path();
    write_canonical_guarded_work_item(dir, WORK_ID, NEW_GUARD, None)?;
    let guard_id = "RFC-0002: C-WORK-WAIVE";
    assert_rejected(
        dir,
        &["work", "waive", WORK_ID, guard_id, "--reason", "x"],
        "error[E1002]",
        guard_id,
        "unknown",
    )?;

    let before = fs::read(work_path(dir))?;
    let output = run_commands(
        dir,
        &[&[
            "--dry-run",
            "work",
            "waive",
            WORK_ID,
            guard_id,
            "--reason",
            "x",
        ]],
    )?;
    assert_eq!(fs::read(work_path(dir))?, before, "output: {output}");
    assert!(
        command_body(&output).contains("error[E1002]"),
        "output: {output}"
    );
    assert!(!output.contains("Would add"), "output: {output}");
    Ok(())
}

#[test]
fn waive_rejects_a_missing_work_item_and_names_both_ids() -> TestResult {
    let temp_dir = project_with_guards(&[NEW_GUARD])?;
    let dir = temp_dir.path();
    let missing_id = "WI-2026-01-01-404";
    let before = work_filenames(dir)?;
    let output = run_commands(
        dir,
        &[&[
            "work",
            "waive",
            missing_id,
            NEW_GUARD,
            "--reason",
            "no such item",
        ]],
    )?;
    assert_eq!(work_filenames(dir)?, before, "output: {output}");
    let diagnostic = command_body(&output);
    assert!(diagnostic.contains("error[E0402]"), "output: {output}");
    assert!(diagnostic.contains(missing_id), "output: {output}");
    assert!(diagnostic.contains(NEW_GUARD), "output: {output}");
    assert!(output.contains("exit: 1"), "output: {output}");
    Ok(())
}

#[test]
fn waive_rejects_a_default_guard_while_verification_is_disabled() -> TestResult {
    let temp_dir = project_with_guards(&["GUARD-DEFAULT", NEW_GUARD])?;
    let dir = temp_dir.path();
    write_item(dir, &[NEW_GUARD], None)?;
    append_verification_config(dir, false, &["GUARD-DEFAULT"])?;
    assert_rejected(
        dir,
        &[
            "work",
            "waive",
            WORK_ID,
            "GUARD-DEFAULT",
            "--reason",
            "project default",
        ],
        "error[E0401]",
        "GUARD-DEFAULT",
        "not required",
    )?;
    Ok(())
}

#[test]
fn waive_accepts_an_enabled_default_guard() -> TestResult {
    let temp_dir = project_with_guards(&["GUARD-DEFAULT", NEW_GUARD])?;
    let dir = temp_dir.path();
    write_item(dir, &[NEW_GUARD], None)?;
    append_verification_config(dir, true, &["GUARD-DEFAULT"])?;

    let output = run_commands(
        dir,
        &[&[
            "work",
            "waive",
            WORK_ID,
            "GUARD-DEFAULT",
            "--reason",
            "project default",
        ]],
    )?;
    assert!(output.contains("exit: 0"), "output: {output}");
    assert_eq!(
        field(dir, "verification.waivers[0].guard")?,
        "GUARD-DEFAULT"
    );
    assert_eq!(
        field(dir, "verification.waivers[0].reason")?,
        "project default"
    );
    Ok(())
}

#[test]
fn waive_dry_run_reports_the_waiver_without_writing() -> TestResult {
    let temp_dir = project_with_guards(&[NEW_GUARD])?;
    let dir = temp_dir.path();
    write_canonical_guarded_work_item(dir, WORK_ID, NEW_GUARD, None)?;
    let before = fs::read(work_path(dir))?;

    let output = run_commands(
        dir,
        &[&[
            "--dry-run",
            "work",
            "waive",
            WORK_ID,
            NEW_GUARD,
            "--reason",
            "  preview reason  ",
        ]],
    )?;

    assert_eq!(fs::read(work_path(dir))?, before, "output: {output}");
    assert!(output.contains("exit: 0"), "output: {output}");
    assert!(
        output.contains(&format!(
            "Would add '{NEW_GUARD}' to {WORK_ID}.verification.waivers with reason 'preview reason'"
        )),
        "output: {output}"
    );
    assert!(!output.contains("Added '"), "output: {output}");
    assert!(!field(dir, "verification.waivers")?.contains("preview reason"));
    Ok(())
}

#[test]
fn work_edit_still_rejects_adding_a_waiver() -> TestResult {
    let temp_dir = project_with_guards(&[NEW_GUARD])?;
    let dir = temp_dir.path();
    write_canonical_guarded_work_item(dir, WORK_ID, NEW_GUARD, None)?;
    assert_rejected(
        dir,
        &[
            "work",
            "edit",
            WORK_ID,
            "verification.waivers",
            "--add",
            NEW_GUARD,
        ],
        "error[E0817]",
        WORK_ID,
        "E0817",
    )?;
    Ok(())
}

#[test]
fn describe_lists_work_waive() -> TestResult {
    let (temp_dir, _) = temp_dir_with_date()?;
    let output = run_commands(temp_dir.path(), &[&["describe"]])?;
    let json = json_object(&output).ok_or("describe did not print a JSON object")?;
    let value: serde_json::Value = serde_json::from_str(json)?;
    let commands = value["commands"]
        .as_array()
        .ok_or("describe commands were not an array")?;
    let work = commands
        .iter()
        .find(|command| command["name"] == "work")
        .ok_or("describe did not list work")?;
    let subcommands = work["subcommands"]
        .as_array()
        .ok_or("work subcommands were not an array")?;
    let waive = subcommands
        .iter()
        .find(|command| command["name"] == "waive")
        .ok_or("describe did not list work waive")?;
    let usage = waive["usage"].as_str().unwrap_or("");
    assert!(
        usage.contains("work waive"),
        "usage: {usage}\noutput: {output}"
    );
    assert!(output.contains("exit: 0"), "output: {output}");
    Ok(())
}

fn work_filenames(dir: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir.join("gov/work"))? {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

fn json_object(output: &str) -> Option<&str> {
    let start = output.find('{')?;
    let end = output.rfind('}')?;
    output.get(start..=end)
}
