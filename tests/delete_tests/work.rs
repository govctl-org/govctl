use super::*;

/// Test: Delete work item - safeguard prevents deleting active work item
#[test]
fn test_delete_work_safeguard_active() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let wi1 = first_work_id(&date);

    // Create active work item
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Active work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "chore: Test criterion".to_string(),
        ],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    assert_delete_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - safeguard prevents deleting done work item
#[test]
fn test_delete_work_safeguard_done() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let wi1 = first_work_id(&date);

    // Create and complete work item
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Completed work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "chore: Test criterion".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "Test".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi1.clone(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    assert_delete_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - successful deletion of queued work item
#[test]
fn test_delete_work_success_queue() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let wi1 = first_work_id(&date);

    // Create two queued work items
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Keep this work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Delete this work item".to_string(),
        ],
        vec!["work".to_string(), "list".to_string()],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
        vec!["work".to_string(), "list".to_string()],
        vec!["check".to_string()],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    assert_delete_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - safeguard prevents deletion when referenced
#[test]
fn test_delete_work_safeguard_referenced() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let wi1 = first_work_id(&date);
    let wi2 = work_id(&date, 2);

    // Create two work items where wi2 references wi1
    let setup_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Referenced work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Work item with reference".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "refs".to_string(),
            wi1.clone(),
        ],
    ];

    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands)?;

    // Try to delete wi1 (should fail because wi2 references it)
    let delete_commands: Vec<Vec<String>> = vec![vec![
        "work".to_string(),
        "delete".to_string(),
        wi1.clone(),
        "-f".to_string(),
    ]];

    let output = run_dynamic_commands(temp_dir.path(), &delete_commands)?;
    assert_delete_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - safeguard prevents deletion when depended on
#[test]
fn test_delete_work_safeguard_depended_on() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let wi1 = first_work_id(&date);
    let wi2 = work_id(&date, 2);

    let setup_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Dependency work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Dependent work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "depends_on".to_string(),
            wi1.clone(),
        ],
    ];

    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands)?;

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ]],
    )?;

    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("Cannot delete work item"),
        "output: {}",
        output
    );
    assert!(output.contains(&wi2), "output: {}", output);
    Ok(())
}
