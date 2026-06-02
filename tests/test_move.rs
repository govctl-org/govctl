//! Tests for the move command - work item status transitions.

mod common;

use common::{init_project, normalize_output, run_commands, run_dynamic_commands, today};
use std::path::Path;
use tempfile::TempDir;

fn init_move_project() -> Result<(TempDir, String), Box<dyn std::error::Error>> {
    let temp_dir = init_project()?;
    let date = today();
    Ok((temp_dir, date))
}

fn first_work_id(date: &str) -> String {
    format!("WI-{date}-001")
}

fn setup_active_work_item_with_criteria(
    dir: &Path,
    date: &str,
    title: &str,
    criterion: &str,
) -> common::TestResult {
    run_dynamic_commands(
        dir,
        &[
            vec![
                "work".to_string(),
                "new".to_string(),
                title.to_string(),
                "--active".to_string(),
            ],
            vec![
                "work".to_string(),
                "add".to_string(),
                first_work_id(date),
                "acceptance_criteria".to_string(),
                criterion.to_string(),
            ],
        ],
    )?;
    Ok(())
}

#[test]
fn test_move_queue_to_active() -> common::TestResult {
    // Move work item from queue to active
    let (temp_dir, date) = init_move_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test task"],
            &["work", "list", "all"],
            &["work", "move", "WI-<DATE>-001", "active"],
            &["work", "list", "all"],
        ],
    )?;
    let output = output.replace("WI-<DATE>-001", &format!("WI-{}-001", date));
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_active_to_done_with_criteria() -> common::TestResult {
    // Move work item from active to done with acceptance criteria
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Test task", "add: Task done")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "work",
                "tick",
                &work_id,
                "acceptance_criteria",
                "Task done",
                "-s",
                "done",
            ],
            &["work", "move", &work_id, "done"],
            &["work", "list", "all"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_to_done_without_criteria_fails() -> common::TestResult {
    // Cannot move to done without acceptance criteria
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test task", "--active"],
            &["work", "move", &work_id, "done"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_to_done_with_pending_criteria_fails() -> common::TestResult {
    // Cannot move to done with pending acceptance criteria
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Test task", "add: Task done")?;

    let output = run_commands(temp_dir.path(), &[&["work", "move", &work_id, "done"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_active_to_cancelled() -> common::TestResult {
    // Move work item from active to cancelled
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Cancelled task", "--active"],
            &["work", "move", &work_id, "cancelled"],
            &["work", "list", "all"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_queue_to_cancelled() -> common::TestResult {
    // Move work item from queue to cancelled (skip active)
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Skipped task"],
            &["work", "move", &work_id, "cancelled"],
            &["work", "list", "all"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_by_work_item_id() -> common::TestResult {
    // Can reference work item by ID instead of filename
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Task with ID ref"],
            &["work", "move", &work_id, "active"],
            &["work", "list", "all"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_nonexistent_work_item() -> common::TestResult {
    // Cannot move non-existent work item
    let (temp_dir, date) = init_move_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-9999-99-999", "active"]],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_sets_started_date() -> common::TestResult {
    // Moving to active sets started date if not already set
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Task to start"],
            &["work", "show", &work_id],
            &["work", "move", &work_id, "active"],
            &["work", "show", &work_id],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_move_sets_completed_date() -> common::TestResult {
    // Moving to done/cancelled sets completed date
    let (temp_dir, date) = init_move_project()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Task to complete", "add: Done")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "work",
                "tick",
                &work_id,
                "acceptance_criteria",
                "Done",
                "-s",
                "done",
            ],
            &["work", "show", &work_id],
            &["work", "move", &work_id, "done"],
            &["work", "show", &work_id],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
