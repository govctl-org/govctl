#[test]
fn test_field_alias_ac() -> common::TestResult {
    // 'ac' should resolve to 'acceptance_criteria'
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "ac",
                "add: Test criterion",
            ],
            &["work", "get", &format!("WI-{}-001", date), "ac"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_desc() -> common::TestResult {
    // 'desc' should resolve to 'description'
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "desc",
                "A description",
            ],
            &["work", "get", &format!("WI-{}-001", date), "desc"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_desc_under_legacy_prefix() -> common::TestResult {
    // content.desc should resolve to description on work items
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "content.desc",
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
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Alias Scope"],
            &["adr", "set", "ADR-0001", "desc", "nope"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
