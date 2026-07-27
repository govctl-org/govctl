use super::*;

#[test]
fn test_field_alias_ac_is_rejected() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "edit",
                &format!("WI-{}-001", date),
                "ac",
                "--add",
                "add: Test criterion",
            ],
            &["work", "get", &format!("WI-{}-001", date), "ac"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_desc_is_rejected() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "edit",
                &format!("WI-{}-001", date),
                "desc",
                "--set",
                "A description",
            ],
            &["work", "get", &format!("WI-{}-001", date), "desc"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_under_storage_prefix_is_rejected() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "edit",
                &format!("WI-{}-001", date),
                "content.desc",
                "--set",
                "Legacy-prefixed description",
            ],
            &["work", "get", &format!("WI-{}-001", date), "description"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_desc_not_global_on_adr() -> common::TestResult {
    // desc is not a valid ADR root field alias and should not be rewritten globally
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Alias Scope"],
            &["adr", "edit", "ADR-0001", "desc", "--set", "nope"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
