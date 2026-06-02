use super::*;

const ADR_ID: &str = "ADR-0001";
const ALTERNATIVES: &str = "alternatives";

fn adr_new(title: &str) -> Vec<String> {
    command(&["adr", "new", title])
}

fn add_alternative(text: &str, pros: &[&str], cons: &[&str]) -> Vec<String> {
    let mut cmd = command(&["adr", "add", ADR_ID, ALTERNATIVES, text]);
    for pro in pros {
        cmd.extend(["--pro".to_string(), (*pro).to_string()]);
    }
    for con in cons {
        cmd.extend(["--con".to_string(), (*con).to_string()]);
    }
    cmd
}

fn adr_get(path: &str) -> Vec<String> {
    command(&["adr", "get", ADR_ID, path])
}

fn adr_set(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "set", ADR_ID, path, value])
}

fn adr_add_path(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "add", ADR_ID, path, value])
}

fn adr_remove(path: &str) -> Vec<String> {
    command(&["adr", "remove", ADR_ID, path])
}

fn adr_remove_match(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "remove", ADR_ID, path, value])
}

fn adr_remove_exact(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "remove", ADR_ID, path, "--exact", value])
}

#[test]
fn test_adr_get_nested_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Path Test"),
            add_alternative("Use traits", &["Flexible", "Reusable"], &["Complex"]),
            adr_get("alt[0].text"),
            adr_get("alt[0].pros"),
            adr_get("alt[0].pros[0]"),
            adr_get("alt[0].cons"),
            adr_get("alternatives[0]"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_set_nested_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Set Test"),
            add_alternative("Option A", &["Fast"], &["Fragile"]),
            adr_set("alt[0].text", "Option A Revised"),
            adr_get("alt[0].text"),
            adr_set("alt[0].pros[0]", "Very fast"),
            adr_get("alt[0].pros[0]"),
            adr_set("alt[0].rejection_reason", "Superseded by Option B"),
            adr_get("alt[0].rejection_reason"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_add_nested_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Add Test"),
            add_alternative("Option X", &["Cheap"], &[]),
            adr_add_path("alt[0].pros", "Reliable"),
            adr_get("alt[0].pros"),
            adr_add_path("alt[0].cons", "Slow"),
            adr_get("alt[0].cons"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_nested_path_rejects_extra_segments() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Depth Test"),
            add_alternative("Option X", &["Fast"], &[]),
            adr_get("alt[0].pros[0].oops"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_add_nested_path_rejects_indexed_terminal() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Indexed Add Test"),
            add_alternative("Option X", &["Fast"], &[]),
            adr_add_path("alt[0].pros[999]", "Ignored"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_get_nested_scalar_rejects_index() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Scalar Index Test"),
            add_alternative("Option X", &["Fast"], &[]),
            adr_get("alt[0].text[0]"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_remove_nested_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Remove Test"),
            add_alternative("Opt1", &["Good", "Great"], &["Bad"]),
            // Remove by sub-index
            adr_remove("alt[0].pros[0]"),
            adr_get("alt[0].pros"),
            // Remove con by pattern match (no terminal index)
            adr_remove_match("alt[0].cons", "Bad"),
            adr_get("alt[0].cons"),
            // Remove entire alternative
            adr_remove("alt[0]"),
            adr_get(ALTERNATIVES),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_remove_nested_path_requires_selector() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Selector Test"),
            add_alternative("Opt1", &[], &["Bad"]),
            adr_remove("alt[0].cons"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_remove_indexed_path_conflict() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Conflict Test"),
            add_alternative("Opt1", &[], &["Bad"]),
            // Indexed path + --exact should produce E0818
            adr_remove_exact("alt[0].cons[0]", "Bad"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
