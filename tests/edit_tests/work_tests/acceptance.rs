use super::*;

const ACCEPTANCE_CRITERIA: &str = "acceptance_criteria";
const CONTENT_ACCEPTANCE_CRITERIA: &str = "content.acceptance_criteria";

fn work_add_acceptance_criteria_with_extras(
    id: &str,
    text: &str,
    category: &str,
    scope: &str,
) -> Vec<String> {
    command(&[
        "work",
        "add",
        id,
        ACCEPTANCE_CRITERIA,
        text,
        "--category",
        category,
        "--scope",
        scope,
    ])
}

fn work_edit_add_content_acceptance_criteria(id: &str, text: &str) -> Vec<String> {
    command(&[
        "work",
        "edit",
        id,
        CONTENT_ACCEPTANCE_CRITERIA,
        "--add",
        text,
    ])
}

fn work_edit_add_acceptance_criteria_with_extras(
    id: &str,
    text: &str,
    category: &str,
    scope: &str,
) -> Vec<String> {
    command(&[
        "work",
        "edit",
        id,
        ACCEPTANCE_CRITERIA,
        "--add",
        text,
        "--category",
        category,
        "--scope",
        scope,
    ])
}

fn work_edit_tick_acceptance_criteria_index(id: &str, index: usize, status: &str) -> Vec<String> {
    let field = format!("{CONTENT_ACCEPTANCE_CRITERIA}[{index}]");
    command(&["work", "edit", id, &field, "--tick", status])
}

#[test]
fn test_work_add_acceptance_criteria() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Test Task"),
            work_add_acceptance(&id, "add: Criterion 1"),
            work_add_acceptance(&id, "add: Criterion 2"),
            work_show(&id),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_add_and_edit_acceptance_criteria_extras() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Category Extras"),
            work_add_acceptance_criteria_with_extras(
                &id,
                "Add without prefix",
                "fixed",
                "legacy-add",
            ),
            work_edit_add_acceptance_criteria_with_extras(
                &id,
                "Edit without prefix",
                "changed",
                "legacy-edit",
            ),
            work_show(&id),
        ],
    )?;

    assert!(
        output.contains("- ○ fixed: Add without prefix"),
        "output: {}",
        output
    );
    assert!(
        output.contains("- ○ changed: Edit without prefix"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_tick_acceptance_criteria() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Test Task"),
            work_add_acceptance(&id, "add: Criterion 1"),
            work_add_acceptance(&id, "add: Criterion 2"),
            work_tick_acceptance(&id, "Criterion 1", "done"),
            work_show(&id),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_edit_tick_indexed_path_canonical() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Canonical Tick"),
            work_edit_add_content_acceptance_criteria(&id, "add: Criterion 1"),
            work_edit_tick_acceptance_criteria_index(&id, 0, "done"),
            work_show(&id),
        ],
    )?;

    assert!(
        output.contains("Added 'add: Criterion 1' to WI-"),
        "output: {}",
        output
    );
    assert!(
        output.contains("Marked 'Criterion 1' as done"),
        "output: {}",
        output
    );
    assert!(
        output.contains("- ✓ added: Criterion 1"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_tick_cancel_acceptance_criteria() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Test Task"),
            work_add_acceptance(&id, "add: Criterion 1"),
            work_tick_acceptance(&id, "Criterion 1", "cancelled"),
            work_show(&id),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_remove_acceptance_criteria() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Test Task"),
            work_add_acceptance(&id, "add: To remove"),
            work_add_acceptance(&id, "add: To keep"),
            work_remove_acceptance(&id, "To remove"),
            work_show(&id),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
