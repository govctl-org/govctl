#[test]
fn test_work_add_acceptance_criteria() -> common::TestResult {
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
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 2",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_add_and_edit_acceptance_criteria_extras() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Category Extras"],
            &[
                "work",
                "add",
                &work_id,
                "acceptance_criteria",
                "Add without prefix",
                "--category",
                "fixed",
                "--scope",
                "legacy-add",
            ],
            &[
                "work",
                "edit",
                &work_id,
                "acceptance_criteria",
                "--add",
                "Edit without prefix",
                "--category",
                "changed",
                "--scope",
                "legacy-edit",
            ],
            &["work", "show", &work_id],
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
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 2",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "Criterion 1",
                "-s",
                "done",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_edit_tick_indexed_path_canonical() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let wi_id = format!("WI-{}-001", date);

    let commands = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Canonical Tick".to_string(),
        ],
        vec![
            "work".to_string(),
            "edit".to_string(),
            wi_id.clone(),
            "content.acceptance_criteria".to_string(),
            "--add".to_string(),
            "add: Criterion 1".to_string(),
        ],
        vec![
            "work".to_string(),
            "edit".to_string(),
            wi_id.clone(),
            "content.acceptance_criteria[0]".to_string(),
            "--tick".to_string(),
            "done".to_string(),
        ],
        vec!["work".to_string(), "show".to_string(), wi_id],
    ];

    let output = common::run_dynamic_commands(temp_dir.path(), &commands)?;

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
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "Criterion 1",
                "-s",
                "cancelled",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_remove_acceptance_criteria() -> common::TestResult {
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
                "acceptance_criteria",
                "add: To remove",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: To keep",
            ],
            &[
                "work",
                "remove",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "To remove",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
