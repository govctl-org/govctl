//! Tests for the move command - work item status transitions.

mod common;

use common::{
    first_work_id, init_project_with_date, normalize_output, run_commands, run_dynamic_commands,
};
use std::path::Path;

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

fn normalize_move_output(
    dir: &Path,
    date: &str,
    output: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = output.replace("WI-<DATE>-001", &first_work_id(date));
    Ok(normalize_output(&output, dir, date)?)
}

macro_rules! assert_move_snapshot {
    ($temp_dir:expr, $date:expr, $output:expr) => {{
        let value = normalize_move_output($temp_dir.path(), $date, $output)?;
        crate::assert_current_test_snapshot!("test_move", value);
        Ok(())
    }};
}

#[test]
fn test_move_queue_to_active() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test task"],
            &["work", "list", "all"],
            &["work", "move", "WI-<DATE>-001", "active"],
            &["work", "list", "all"],
        ],
    )?;
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_active_to_done_with_criteria() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
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
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_to_done_without_criteria_fails() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test task", "--active"],
            &["work", "move", &work_id, "done"],
        ],
    )?;
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_to_done_with_pending_criteria_fails() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Test task", "add: Task done")?;

    let output = run_commands(temp_dir.path(), &[&["work", "move", &work_id, "done"]])?;
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_active_to_cancelled() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Cancelled task", "--active"],
            &["work", "move", &work_id, "cancelled"],
            &["work", "list", "all"],
        ],
    )?;
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_queue_to_cancelled() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Skipped task"],
            &["work", "move", &work_id, "cancelled"],
            &["work", "list", "all"],
        ],
    )?;
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_by_work_item_id() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Task with ID ref"],
            &["work", "move", &work_id, "active"],
            &["work", "list", "all"],
        ],
    )?;
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_nonexistent_work_item() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-9999-99-999", "active"]],
    )?;
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_sets_started_date() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
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
    assert_move_snapshot!(temp_dir, &date, &output)
}

#[test]
fn test_move_sets_completed_date() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
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
    assert_move_snapshot!(temp_dir, &date, &output)
}
