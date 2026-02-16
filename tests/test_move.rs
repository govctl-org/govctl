//! Tests for the move command - work item status transitions.

mod common;

use common::{init_project, normalize_output, run_commands, run_dynamic_commands, today};

#[test]
fn test_move_queue_to_active() {
    // Move work item from queue to active
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test task"],
            &["work", "list", "all"],
            &["work", "move", "WI-<DATE>-001", "active"],
            &["work", "list", "all"],
        ],
    );
    let output = output.replace("WI-<DATE>-001", &format!("WI-{}-001", date));
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_active_to_done_with_criteria() {
    // Move work item from active to done with acceptance criteria
    let temp_dir = init_project();
    let date = today();

    // Create and set up work item
    let setup_commands: Vec<Vec<String>> = vec![
        vec!["work".to_string(), "new".to_string(), "Test task".to_string(), "--active".to_string()],
        vec!["work".to_string(), "add".to_string(), format!("WI-{}-001", date), "acceptance_criteria".to_string(), "add: Task done".to_string()],
    ];
    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "tick", &format!("WI-{}-001", date), "acceptance_criteria", "Task done", "-s", "done"],
            &["work", "move", &format!("WI-{}-001", date), "done"],
            &["work", "list", "all"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_to_done_without_criteria_fails() {
    // Cannot move to done without acceptance criteria
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test task", "--active"],
            &["work", "move", &format!("WI-{}-001", date), "done"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_to_done_with_pending_criteria_fails() {
    // Cannot move to done with pending acceptance criteria
    let temp_dir = init_project();
    let date = today();

    let setup_commands: Vec<Vec<String>> = vec![
        vec!["work".to_string(), "new".to_string(), "Test task".to_string(), "--active".to_string()],
        vec!["work".to_string(), "add".to_string(), format!("WI-{}-001", date), "acceptance_criteria".to_string(), "add: Task done".to_string()],
    ];
    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "move", &format!("WI-{}-001", date), "done"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_active_to_cancelled() {
    // Move work item from active to cancelled
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Cancelled task", "--active"],
            &["work", "move", &format!("WI-{}-001", date), "cancelled"],
            &["work", "list", "all"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_queue_to_cancelled() {
    // Move work item from queue to cancelled (skip active)
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Skipped task"],
            &["work", "move", &format!("WI-{}-001", date), "cancelled"],
            &["work", "list", "all"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_by_work_item_id() {
    // Can reference work item by ID instead of filename
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Task with ID ref"],
            &["work", "move", &format!("WI-{}-001", date), "active"],
            &["work", "list", "all"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_nonexistent_work_item() {
    // Cannot move non-existent work item
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-9999-99-999", "active"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_sets_started_date() {
    // Moving to active sets started date if not already set
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Task to start"],
            &["work", "show", &format!("WI-{}-001", date)],
            &["work", "move", &format!("WI-{}-001", date), "active"],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_move_sets_completed_date() {
    // Moving to done/cancelled sets completed date
    let temp_dir = init_project();
    let date = today();

    let setup_commands: Vec<Vec<String>> = vec![
        vec!["work".to_string(), "new".to_string(), "Task to complete".to_string(), "--active".to_string()],
        vec!["work".to_string(), "add".to_string(), format!("WI-{}-001", date), "acceptance_criteria".to_string(), "add: Done".to_string()],
    ];
    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "tick", &format!("WI-{}-001", date), "acceptance_criteria", "Done", "-s", "done"],
            &["work", "show", &format!("WI-{}-001", date)],
            &["work", "move", &format!("WI-{}-001", date), "done"],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
