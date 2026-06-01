#[test]
fn test_work_get_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "get", &format!("WI-{}-001", date), "title"],
            &["work", "get", &format!("WI-{}-001", date), "status"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_set_title() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Original Title"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "title",
                "New Title",
            ],
            &["work", "list", "all"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_set_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "set", &work_id, "status", "active"],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work move"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_set_acceptance_criteria_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &work_id,
                "acceptance_criteria",
                "add: Test criterion",
            ],
            &[
                "work",
                "set",
                &work_id,
                "acceptance_criteria[0].status",
                "done",
            ],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work tick"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_get_nonexistent() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "get", "WI-9999-99-999", "title"]],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
