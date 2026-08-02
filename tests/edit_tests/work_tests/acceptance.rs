use super::*;

const ACCEPTANCE_CRITERIA: &str = "acceptance_criteria";

fn work_edit_tick_acceptance_criteria_index(id: &str, index: usize, status: &str) -> Vec<String> {
    let field = format!("{ACCEPTANCE_CRITERIA}[{index}]");
    command(&["work", "edit", id, &field, "--tick", status])
}

fn read_first_criterion(
    project: &std::path::Path,
) -> Result<toml::value::Table, Box<dyn std::error::Error>> {
    let work_path = std::fs::read_dir(project.join("gov/work"))?
        .next()
        .ok_or("work item file missing")??
        .path();
    let work: toml::Value = toml::from_str(&std::fs::read_to_string(work_path)?)?;
    work.get("content")
        .and_then(|value| value.get("acceptance_criteria"))
        .and_then(toml::Value::as_array)
        .and_then(|criteria| criteria.first())
        .and_then(toml::Value::as_table)
        .cloned()
        .ok_or_else(|| "first acceptance criterion missing".into())
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
fn test_work_acceptance_criteria_use_category_prefixes() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Category Extras"),
            work_add_acceptance(&id, "fixed: Add with prefix"),
            work_add_acceptance(&id, "changed: Edit with prefix"),
            work_show(&id),
        ],
    )?;

    assert!(
        output.contains("- ○ fixed: Add with prefix"),
        "output: {}",
        output
    );
    assert!(
        output.contains("- ○ changed: Edit with prefix"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_set_acceptance_criterion_parses_prefix_and_preserves_status() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Correct Criterion"),
            work_add_acceptance(&id, "add: Original text"),
            work_tick_acceptance(&id, 0, "done"),
            work_set_field(&id, "acceptance_criteria[0]", "fix: Corrected text"),
        ],
    )?;

    let criterion = read_first_criterion(temp_dir.path())?;
    assert_eq!(
        criterion.get("text").and_then(toml::Value::as_str),
        Some("Corrected text")
    );
    assert_eq!(
        criterion.get("category").and_then(toml::Value::as_str),
        Some("fixed")
    );
    assert_eq!(
        criterion.get("status").and_then(toml::Value::as_str),
        Some("done")
    );
    Ok(())
}

#[test]
fn test_work_set_acceptance_criterion_falls_back_to_text_and_preserves_metadata()
-> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Correct Criterion"),
            work_add_acceptance(&id, "change: Original text"),
            work_tick_acceptance(&id, 0, "cancelled"),
            work_set_field(
                &id,
                "acceptance_criteria[0]",
                "API: Preserve response shape",
            ),
        ],
    )?;

    let criterion = read_first_criterion(temp_dir.path())?;
    assert_eq!(
        criterion.get("text").and_then(toml::Value::as_str),
        Some("API: Preserve response shape")
    );
    assert_eq!(
        criterion.get("category").and_then(toml::Value::as_str),
        Some("changed")
    );
    assert_eq!(
        criterion.get("status").and_then(toml::Value::as_str),
        Some("cancelled")
    );
    Ok(())
}

#[test]
fn test_work_set_acceptance_criterion_text_path_is_literal() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Correct Criterion"),
            work_add_acceptance(&id, "add: Original text"),
            work_tick_acceptance(&id, 0, "done"),
            work_set_field(&id, "acceptance_criteria[0].text", "fix: Literal text"),
        ],
    )?;

    let criterion = read_first_criterion(temp_dir.path())?;
    assert_eq!(
        criterion.get("text").and_then(toml::Value::as_str),
        Some("fix: Literal text")
    );
    assert_eq!(
        criterion.get("category").and_then(toml::Value::as_str),
        Some("added")
    );
    assert_eq!(
        criterion.get("status").and_then(toml::Value::as_str),
        Some("done")
    );
    Ok(())
}

#[test]
fn test_work_set_acceptance_criterion_rejects_invalid_value_and_index_atomically()
-> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Correct Criterion"),
            work_add_acceptance(&id, "add: Original text"),
            work_set_field(&id, "acceptance_criteria[0]", ""),
            work_set_field(&id, "acceptance_criteria[0].text", ""),
            work_set_field(&id, "acceptance_criteria[4]", "fix: Missing item"),
        ],
    )?;

    assert_eq!(
        output.matches("error[E0805]").count(),
        2,
        "output: {output}"
    );
    assert!(output.contains("error[E0816]"), "output: {output}");
    let criterion = read_first_criterion(temp_dir.path())?;
    assert_eq!(
        criterion.get("text").and_then(toml::Value::as_str),
        Some("Original text")
    );
    assert_eq!(
        criterion.get("category").and_then(toml::Value::as_str),
        Some("added")
    );
    assert_eq!(
        criterion.get("status").and_then(toml::Value::as_str),
        Some("pending")
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
            work_tick_acceptance(&id, 0, "done"),
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
            work_add_acceptance(&id, "add: Criterion 1"),
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
            work_tick_acceptance(&id, 0, "cancelled"),
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
