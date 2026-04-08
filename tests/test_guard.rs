//! Guard resource subcommand tests.

mod common;

use common::{init_project, run_commands};
use std::fs;
use std::path::Path;

#[test]
fn test_guard_new_scaffolds_file() {
    let temp_dir = init_project();

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "clippy lint"]]);
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-CLIPPY-LINT"), "output: {}", output);
    assert!(
        output.contains("verification.default_guards"),
        "should hint about default_guards: {}",
        output
    );

    let guard_path = temp_dir.path().join("gov/guard/clippy-lint.toml");
    assert!(guard_path.exists(), "guard file should be created");

    let content = fs::read_to_string(&guard_path).unwrap();
    assert!(content.contains("GUARD-CLIPPY-LINT"));
    assert!(content.contains("clippy lint"));
    assert!(content.contains("[check]"));
}

#[test]
fn test_guard_new_duplicate_rejected() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ECHO", "true");

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "echo"]]);
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("already exists"),
        "should reject duplicate: {}",
        output
    );
}

#[test]
fn test_guard_new_invalid_title_rejected_with_code() {
    let temp_dir = init_project();

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "123 !!!"]]);
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1006]"), "output: {}", output);
}

#[test]
fn test_guard_list() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ALPHA", "true");
    write_guard(temp_dir.path(), "GUARD-BETA", "echo ok");

    let output = run_commands(temp_dir.path(), &[&["guard", "list"]]);
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-ALPHA"), "output: {}", output);
    assert!(output.contains("GUARD-BETA"), "output: {}", output);
}

#[test]
fn test_guard_show() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello");

    let output = run_commands(temp_dir.path(), &[&["guard", "show", "GUARD-ECHO"]]);
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-ECHO"), "output: {}", output);
    assert!(output.contains("echo hello"), "output: {}", output);
}

#[test]
fn test_guard_show_json_output() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello");

    let output = run_commands(
        temp_dir.path(),
        &[&["guard", "show", "GUARD-ECHO", "-o", "json"]],
    );
    assert!(
        output.contains("\"id\": \"GUARD-ECHO\""),
        "output: {}",
        output
    );
    assert!(
        output.contains("\"command\": \"echo hello\""),
        "output: {}",
        output
    );
    assert!(output.contains("exit: 0"), "output: {}", output);
}

#[test]
fn test_guard_show_missing_returns_coded_error() {
    let temp_dir = init_project();

    let output = run_commands(temp_dir.path(), &[&["guard", "show", "GUARD-MISSING"]]);
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1002]"), "output: {}", output);
}

#[test]
fn test_guard_get_field() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello");

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "get", "GUARD-ECHO", "command"],
            &["guard", "get", "GUARD-ECHO", "title"],
        ],
    );
    assert!(output.contains("echo hello"), "output: {}", output);
    assert!(output.contains("GUARD-ECHO"), "output: {}", output);
}

#[test]
fn test_guard_set_command() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo old");

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "set", "GUARD-ECHO", "command", "echo new"],
            &["guard", "get", "GUARD-ECHO", "command"],
        ],
    );
    assert!(output.contains("echo new"), "output: {}", output);
}

#[test]
fn test_guard_delete_unreferenced() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-TEMP", "true");

    let guard_path = temp_dir.path().join("gov/guard/guard-temp.toml");
    assert!(guard_path.exists());

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-TEMP"]]);
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(!guard_path.exists(), "guard file should be deleted");
}

#[test]
fn test_guard_delete_blocked_by_config() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-IMPORTANT", "true");
    append_verification_config(temp_dir.path(), true, &["GUARD-IMPORTANT"]);

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-IMPORTANT"]]);
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "should block delete: {}",
        output
    );
    assert!(
        output.contains("default_guards"),
        "should mention config: {}",
        output
    );
}

#[test]
fn test_guard_delete_blocked_by_work_item() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-REQUIRED", "true");
    write_work_item_with_guard(temp_dir.path(), "GUARD-REQUIRED");

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-REQUIRED"]]);
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "should block delete: {}",
        output
    );
}

#[test]
fn test_guard_delete_force_does_not_bypass_reference_checks() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-FORCED", "true");
    append_verification_config(temp_dir.path(), true, &["GUARD-FORCED"]);

    let output = run_commands(
        temp_dir.path(),
        &[&["guard", "delete", "GUARD-FORCED", "--force"]],
    );
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "force should not bypass reference checks: {}",
        output
    );
}

#[test]
fn test_guard_delete_missing_returns_coded_error() {
    let temp_dir = init_project();

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-MISSING"]]);
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1002]"), "output: {}", output);
}

#[test]
fn test_guard_set_timeout_secs() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ECHO", "true");

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "set", "GUARD-ECHO", "timeout_secs", "30"],
            &["guard", "get", "GUARD-ECHO", "timeout_secs"],
        ],
    );
    assert!(output.contains("30"), "output: {}", output);
}

#[test]
fn test_guard_nested_object_root_edit_paths() {
    let temp_dir = init_project();
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo old");

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "guard",
                "set",
                "GUARD-ECHO",
                "check.command",
                "echo nested legacy",
            ],
            &[
                "guard",
                "edit",
                "GUARD-ECHO",
                "check.timeout_secs",
                "--set",
                "45",
            ],
            &["guard", "get", "GUARD-ECHO", "command"],
            &["guard", "get", "GUARD-ECHO", "timeout_secs"],
        ],
    );

    assert!(
        output.contains("Set GUARD-ECHO.check.command = echo nested legacy"),
        "output: {}",
        output
    );
    assert!(
        output.contains("Set GUARD-ECHO.check.timeout_secs = 45"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl guard get GUARD-ECHO command\necho nested legacy"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl guard get GUARD-ECHO timeout_secs\n45"),
        "output: {}",
        output
    );
}

// --- Helpers ---

fn write_guard(dir: &Path, guard_id: &str, command: &str) {
    let path = dir
        .join("gov/guard")
        .join(format!("{}.toml", guard_id.to_lowercase()));
    let content = format!(
        "#:schema ../schema/guard.schema.json\n\n[govctl]\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\n"
    );
    fs::write(path, content).expect("guard file should be writable");
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

fn write_work_item_with_guard(dir: &Path, guard_id: &str) {
    let path = dir.join("gov/work/2026-01-01-guarded-item.toml");
    let content = format!(
        "#:schema ../schema/work.schema.json\n\n[govctl]\nid = \"WI-2026-01-01-001\"\ntitle = \"Guarded Item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Guarded work item\"\n\n[[content.acceptance_criteria]]\ntext = \"done\"\nstatus = \"done\"\ncategory = \"chore\"\n\n[verification]\nrequired_guards = [\"{guard_id}\"]\n"
    );
    fs::write(path, content).expect("work item should be writable");
}
