use super::*;

/// Test: Delete work item - safeguard prevents deleting active work item
#[test]
fn test_delete_work_safeguard_active() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let wi1 = first_work_id(&date);

    // Create active work item
    let commands: Vec<Vec<String>> = vec![
        work_new_active("Active work item"),
        work_add_acceptance(&wi1, "chore: Test criterion"),
        work_delete_force(&wi1),
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
        work_new_active("Completed work item"),
        work_add_acceptance(&wi1, "chore: Test criterion"),
        work_tick_acceptance_done(&wi1, "Test"),
        work_move_done(&wi1),
        work_delete_force(&wi1),
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
        work_new("Keep this work item"),
        work_new("Delete this work item"),
        command(&["work", "list"]),
        work_delete_force(&wi1),
        command(&["work", "list"]),
        command(&["check"]),
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
        work_new("Referenced work item"),
        work_new("Work item with reference"),
        work_add_field(&wi2, "refs", &wi1),
    ];

    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands)?;

    // Try to delete wi1 (should fail because wi2 references it)
    let delete_commands: Vec<Vec<String>> = vec![work_delete_force(&wi1)];

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
        work_new("Dependency work item"),
        work_new("Dependent work item"),
        work_add_dependency(&wi2, &wi1),
    ];

    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands)?;

    let output = run_dynamic_commands(temp_dir.path(), &[work_delete_force(&wi1)])?;

    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("Cannot delete work item"),
        "output: {}",
        output
    );
    assert!(output.contains(&wi2), "output: {}", output);
    Ok(())
}
