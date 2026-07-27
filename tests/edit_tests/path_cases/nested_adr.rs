use super::*;

const ADR_ID: &str = "ADR-0001";
const ALTERNATIVES: &str = "alternatives";

fn adr_new(title: &str) -> Vec<String> {
    command(&["adr", "new", title])
}

fn add_alternative(text: &str) -> Vec<String> {
    command(&["adr", "edit", ADR_ID, ALTERNATIVES, "--add", text])
}

fn adr_get(path: &str) -> Vec<String> {
    command(&["adr", "get", ADR_ID, path])
}

fn adr_set(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "edit", ADR_ID, path, "--set", value])
}

fn adr_add_path(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "edit", ADR_ID, path, "--add", value])
}

fn adr_remove(path: &str) -> Vec<String> {
    command(&["adr", "edit", ADR_ID, path, "--remove"])
}

fn adr_remove_match(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "edit", ADR_ID, path, "--remove", value])
}

fn adr_remove_exact(path: &str, value: &str) -> Vec<String> {
    command(&["adr", "edit", ADR_ID, path, "--remove", value])
}

#[test]
fn test_adr_get_nested_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            adr_new("Path Test"),
            add_alternative("Use traits"),
            adr_add_path("alternatives[0].pros", "Flexible"),
            adr_add_path("alternatives[0].pros", "Reusable"),
            adr_add_path("alternatives[0].cons", "Complex"),
            adr_get("alternatives[0].text"),
            adr_get("alternatives[0].pros"),
            adr_get("alternatives[0].pros[0]"),
            adr_get("alternatives[0].cons"),
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
            add_alternative("Option A"),
            adr_add_path("alternatives[0].pros", "Fast"),
            adr_add_path("alternatives[0].cons", "Fragile"),
            adr_set("alternatives[0].text", "Option A Revised"),
            adr_get("alternatives[0].text"),
            adr_set("alternatives[0].pros[0]", "Very fast"),
            adr_get("alternatives[0].pros[0]"),
            adr_set("alternatives[0].rejection_reason", "Superseded by Option B"),
            adr_get("alternatives[0].rejection_reason"),
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
            add_alternative("Option X"),
            adr_add_path("alternatives[0].pros", "Cheap"),
            adr_add_path("alternatives[0].pros", "Reliable"),
            adr_get("alternatives[0].pros"),
            adr_add_path("alternatives[0].cons", "Slow"),
            adr_get("alternatives[0].cons"),
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
            add_alternative("Option X"),
            adr_add_path("alternatives[0].pros", "Fast"),
            adr_get("alternatives[0].pros[0].oops"),
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
            add_alternative("Option X"),
            adr_add_path("alternatives[0].pros", "Fast"),
            adr_add_path("alternatives[0].pros[999]", "Ignored"),
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
            add_alternative("Option X"),
            adr_add_path("alternatives[0].pros", "Fast"),
            adr_get("alternatives[0].text[0]"),
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
            add_alternative("Opt1"),
            adr_add_path("alternatives[0].pros", "Good"),
            adr_add_path("alternatives[0].pros", "Great"),
            adr_add_path("alternatives[0].cons", "Bad"),
            // Remove by sub-index
            adr_remove("alternatives[0].pros[0]"),
            adr_get("alternatives[0].pros"),
            // Remove con by pattern match (no terminal index)
            adr_remove_match("alternatives[0].cons", "Bad"),
            adr_get("alternatives[0].cons"),
            // Remove entire alternative
            adr_remove("alternatives[0]"),
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
            add_alternative("Opt1"),
            adr_add_path("alternatives[0].cons", "Bad"),
            adr_remove("alternatives[0].cons"),
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
            add_alternative("Opt1"),
            adr_add_path("alternatives[0].cons", "Bad"),
            // Indexed path plus a remove value is conflicting.
            adr_remove_exact("alternatives[0].cons[0]", "Bad"),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
