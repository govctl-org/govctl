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
