use super::*;

const ACCEPTANCE_CRITERIA_STATUS: &str = "acceptance_criteria[0].status";
const STATUS: &str = "status";
const TITLE: &str = "title";

#[test]
fn test_work_get_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Test Task"),
            work_get_field(&id, TITLE),
            work_get_field(&id, STATUS),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_set_title() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Original Title"),
            work_set_field(&id, TITLE, "New Title"),
            work_list_all(),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_get_falls_back_to_partial_filename_starting_with_wi() -> common::TestResult {
    let temp_dir = init_project()?;
    std::fs::write(
        temp_dir.path().join("gov/work/WI-partial-edit.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-778"
title = "Partial Edit Lookup"
status = "queue"
created = "2026-01-01"

[content]
description = "Partial edit lookup"
"#,
    )?;

    let output = common::run_commands(temp_dir.path(), &[&["work", "get", "WI-partial", "title"]])?;
    assert!(output.contains("Partial Edit Lookup"), "{output}");
    Ok(())
}

#[test]
fn test_work_get_by_id_propagates_lookup_errors() -> common::TestResult {
    let temp_dir = init_project()?;
    let work_dir = temp_dir.path().join("gov/work");
    std::fs::remove_dir_all(&work_dir)?;
    std::fs::write(&work_dir, "not a directory")?;

    let output = common::run_commands(
        temp_dir.path(),
        &[&["work", "get", "WI-2026-01-01-001", "title"]],
    )?;
    assert!(output.contains("error[E0901]"), "{output}");
    assert!(!output.contains("Work item not found"), "{output}");
    Ok(())
}

#[test]
fn test_work_set_status_rejected() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[work_new("Test Task"), work_set_field(&id, STATUS, "active")],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work move"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_set_acceptance_criteria_status_rejected() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Test Task"),
            work_add_acceptance(&id, "add: Test criterion"),
            work_set_field(&id, ACCEPTANCE_CRITERIA_STATUS, "done"),
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work tick"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_get_nonexistent() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output =
        common::run_dynamic_commands(temp_dir.path(), &[work_get_field("WI-9999-99-999", TITLE)])?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
