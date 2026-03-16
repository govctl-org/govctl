//! Verification guard integration tests.

mod common;

use common::{init_project, run_commands};
use std::fs;
use std::path::Path;

#[test]
fn test_verify_runs_project_default_guard() {
    let temp_dir = init_project();

    append_verification_config(temp_dir.path(), true, &["GUARD-ECHO"]);
    write_guard(temp_dir.path(), "GUARD-ECHO", "true", None);

    let output = run_commands(temp_dir.path(), &[&["verify"]]);
    assert!(output.contains("PASS GUARD-ECHO"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);
}

#[test]
fn test_work_move_done_rejects_failed_required_guard() {
    let temp_dir = init_project();

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1", None);
    write_active_work_item(temp_dir.path(), "WI-2026-01-01-001", "GUARD-FAIL", None);

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-2026-01-01-001", "done"]],
    );
    assert!(
        output.contains("Cannot mark as done: verification guard requirements failed"),
        "output: {}",
        output
    );
    assert!(output.contains("error[E1004]"), "output: {}", output);
    assert!(output.contains("exit: 1"), "output: {}", output);
}

#[test]
fn test_work_move_done_allows_waived_guard() {
    let temp_dir = init_project();

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1", None);
    write_active_work_item(
        temp_dir.path(),
        "WI-2026-01-01-001",
        "GUARD-FAIL",
        Some("Guard does not apply to this work item."),
    );

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-2026-01-01-001", "done"]],
    );
    assert!(output.contains("exit: 0"), "output: {}", output);

    let work_file = temp_dir
        .path()
        .join("gov/work/2026-01-01-guarded-item.toml");
    let content = fs::read_to_string(&work_file).expect("work item should be readable");
    assert!(
        content.contains("status = \"done\""),
        "content: {}",
        content
    );
}

fn append_verification_config(dir: &Path, enabled: bool, guard_ids: &[&str]) {
    let config_path = dir.join("gov/config.toml");
    let existing = fs::read_to_string(&config_path).expect("config should exist");
    let default_guards = guard_ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let appended = format!(
        "{existing}\n[verification]\nenabled = {enabled}\ndefault_guards = [{default_guards}]\n"
    );
    fs::write(config_path, appended).expect("config should be writable");
}

fn write_guard(dir: &Path, guard_id: &str, command: &str, pattern: Option<&str>) {
    let path = dir
        .join("gov/guard")
        .join(format!("{}.toml", guard_id.to_lowercase()));
    let pattern_line = pattern
        .map(|pattern| format!("pattern = \"{pattern}\"\n"))
        .unwrap_or_default();
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\n{pattern_line}"
    );
    fs::write(path, content).expect("guard file should be writable");
}

fn write_active_work_item(dir: &Path, work_id: &str, guard_id: &str, waiver_reason: Option<&str>) {
    let path = dir.join("gov/work/2026-01-01-guarded-item.toml");
    let waiver = waiver_reason
        .map(|reason| {
            format!("\n[[verification.waivers]]\nguard = \"{guard_id}\"\nreason = \"{reason}\"\n")
        })
        .unwrap_or_default();
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{work_id}\"\ntitle = \"Guarded Item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Guarded work item\"\n\n[[content.acceptance_criteria]]\ntext = \"done criteria\"\nstatus = \"done\"\ncategory = \"chore\"\n\n[verification]\nrequired_guards = [\"{guard_id}\"]{waiver}"
    );
    fs::write(path, content).expect("work item should be writable");
}
