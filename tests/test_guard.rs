//! Guard resource subcommand tests.

mod common;

use common::{init_project, run_commands};
use std::fs;
use std::path::Path;

#[test]
fn test_guard_new_scaffolds_file() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "clippy lint"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-CLIPPY-LINT"), "output: {}", output);
    assert!(
        output.contains("verification.default_guards"),
        "should hint about default_guards: {}",
        output
    );

    let guard_path = temp_dir.path().join("gov/guard/clippy-lint.toml");
    assert!(guard_path.exists(), "guard file should be created");

    let content = fs::read_to_string(&guard_path)?;
    assert!(content.contains("GUARD-CLIPPY-LINT"));
    assert!(content.contains("clippy lint"));
    assert!(content.contains("[check]"));
    Ok(())
}

#[test]
fn test_guard_new_duplicate_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "true")?;

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "echo"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("already exists"),
        "should reject duplicate: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_new_invalid_title_rejected_with_code() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "123 !!!"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1006]"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_list() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ALPHA", "true")?;
    write_guard(temp_dir.path(), "GUARD-BETA", "echo ok")?;

    let output = run_commands(temp_dir.path(), &[&["guard", "list"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-ALPHA"), "output: {}", output);
    assert!(output.contains("GUARD-BETA"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_show() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello")?;

    let output = run_commands(temp_dir.path(), &[&["guard", "show", "GUARD-ECHO"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-ECHO"), "output: {}", output);
    assert!(output.contains("echo hello"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_show_json_output() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello")?;

    let output = run_commands(
        temp_dir.path(),
        &[&["guard", "show", "GUARD-ECHO", "-o", "json"]],
    )?;
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
    Ok(())
}

#[test]
fn test_guard_show_missing_returns_coded_error() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "show", "GUARD-MISSING"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1002]"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_get_field() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "get", "GUARD-ECHO", "command"],
            &["guard", "get", "GUARD-ECHO", "title"],
        ],
    )?;
    assert!(output.contains("echo hello"), "output: {}", output);
    assert!(output.contains("GUARD-ECHO"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_set_command() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo old")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "set", "GUARD-ECHO", "command", "echo new"],
            &["guard", "get", "GUARD-ECHO", "command"],
        ],
    )?;
    assert!(output.contains("echo new"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_delete_unreferenced() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-TEMP", "true")?;

    let guard_path = temp_dir.path().join("gov/guard/guard-temp.toml");
    assert!(guard_path.exists());

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-TEMP"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(!guard_path.exists(), "guard file should be deleted");
    Ok(())
}

#[test]
fn test_guard_delete_blocked_by_config() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-IMPORTANT", "true")?;
    append_verification_config(temp_dir.path(), true, &["GUARD-IMPORTANT"])?;

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-IMPORTANT"]])?;
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
    Ok(())
}

#[test]
fn test_guard_delete_blocked_by_work_item() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-REQUIRED", "true")?;
    write_work_item_with_guard(temp_dir.path(), "GUARD-REQUIRED")?;

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-REQUIRED"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "should block delete: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_delete_force_does_not_bypass_reference_checks() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-FORCED", "true")?;
    append_verification_config(temp_dir.path(), true, &["GUARD-FORCED"])?;

    let output = run_commands(
        temp_dir.path(),
        &[&["guard", "delete", "GUARD-FORCED", "--force"]],
    )?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "force should not bypass reference checks: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_delete_missing_returns_coded_error() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-MISSING"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1002]"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_set_timeout_secs() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "true")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "set", "GUARD-ECHO", "timeout_secs", "30"],
            &["guard", "get", "GUARD-ECHO", "timeout_secs"],
        ],
    )?;
    assert!(output.contains("30"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_nested_object_root_edit_paths() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo old")?;

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
    )?;

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
    Ok(())
}

// --- Helpers ---

fn write_guard(
    dir: &Path,
    guard_id: &str,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir
        .join("gov/guard")
        .join(format!("{}.toml", guard_id.to_lowercase()));
    let content = format!(
        "#:schema ../schema/guard.schema.json\n\n[govctl]\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\n"
    );
    fs::write(path, content)?;
    Ok(())
}

fn append_verification_config(
    dir: &Path,
    enabled: bool,
    guard_ids: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let existing = fs::read_to_string(&config_path)?;
    let default_guards = guard_ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let appended = format!(
        "{existing}\n[verification]\nenabled = {enabled}\ndefault_guards = [{default_guards}]\n"
    );
    fs::write(config_path, appended)?;
    Ok(())
}

fn write_work_item_with_guard(
    dir: &Path,
    guard_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join("gov/work/2026-01-01-guarded-item.toml");
    let content = format!(
        "#:schema ../schema/work.schema.json\n\n[govctl]\nid = \"WI-2026-01-01-001\"\ntitle = \"Guarded Item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Guarded work item\"\n\n[[content.acceptance_criteria]]\ntext = \"done\"\nstatus = \"done\"\ncategory = \"chore\"\n\n[verification]\nrequired_guards = [\"{guard_id}\"]\n"
    );
    fs::write(path, content)?;
    Ok(())
}
