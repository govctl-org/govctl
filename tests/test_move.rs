//! Tests for the move command - work item status transitions.

mod common;

use common::{
    first_work_id, init_project, init_project_with_date, normalize_output, run_commands,
    run_dynamic_commands, work_add_acceptance, work_new_active,
};
use std::{fs, path::Path};

fn setup_active_work_item_with_criteria(
    dir: &Path,
    date: &str,
    title: &str,
    criterion: &str,
) -> common::TestResult {
    run_dynamic_commands(
        dir,
        &[
            work_new_active(title),
            work_add_acceptance(&first_work_id(date), criterion),
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
fn test_move_falls_back_to_partial_filename_starting_with_wi() -> common::TestResult {
    let temp_dir = init_project()?;
    let work_path = temp_dir.path().join("gov/work/WI-partial-custom.toml");
    fs::write(
        work_path,
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-777"
title = "Partial Filename"
status = "active"
created = "2026-01-01"
started = "2026-01-01"

[content]
description = "Partial filename lookup"
"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "move", "WI-partial", "cancelled"],
            &["work", "get", "WI-2026-01-01-777", "status"],
        ],
    )?;
    assert!(output.contains("Moved WI-partial-custom.toml to cancelled"));
    assert!(output.contains("$ govctl work get WI-2026-01-01-777 status\ncancelled"));
    Ok(())
}

#[test]
fn test_move_by_work_item_id_propagates_lookup_errors() -> common::TestResult {
    let temp_dir = init_project()?;
    let work_dir = temp_dir.path().join("gov/work");
    fs::remove_dir_all(&work_dir)?;
    fs::write(&work_dir, "not a directory")?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-2026-01-01-001", "active"]],
    )?;
    assert!(output.contains("error[E0901]"), "{output}");
    assert!(!output.contains("Work item not found"), "{output}");
    Ok(())
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

#[test]
fn test_move_reopens_unreleased_done_item_and_updates_timestamps() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Task", "add: Done")?;
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
            &["work", "move", &work_id, "done"],
            &["work", "move", &work_id, "active"],
            &["work", "get", &work_id, "status"],
            &["work", "get", &work_id, "started"],
        ],
    )?;

    assert!(
        output.contains(&format!("$ govctl work get {work_id} status\nactive")),
        "output: {output}"
    );
    assert!(
        output.contains(&format!("$ govctl work get {work_id} started\n{date}")),
        "output: {output}"
    );

    let work_path = fs::read_dir(temp_dir.path().join("gov/work"))?
        .next()
        .ok_or("missing work item")??
        .path();
    let work: toml::Value = toml::from_str(&fs::read_to_string(work_path)?)?;
    assert!(work["govctl"].get("completed").is_none());
    Ok(())
}

#[test]
fn test_move_rejects_reopening_released_done_item() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Task", "add: Done")?;
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
            &["work", "move", &work_id, "done"],
            &["release", "0.1.0"],
            &["work", "move", &work_id, "active"],
            &["work", "get", &work_id, "status"],
        ],
    )?;

    assert!(output.contains("error[E0403]"), "output: {output}");
    assert!(
        output.contains("Cannot reopen a Work Item referenced by a release"),
        "output: {output}"
    );
    assert!(
        output.contains(&format!("$ govctl work get {work_id} status\ndone")),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_move_reopen_release_load_failure_preserves_work_item() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Task", "add: Done")?;
    run_commands(
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
            &["work", "move", &work_id, "done"],
        ],
    )?;

    let work_path = fs::read_dir(temp_dir.path().join("gov/work"))?
        .next()
        .ok_or("missing work item")??
        .path();
    let original = fs::read_to_string(&work_path)?;
    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        "not valid TOML [",
    )?;

    let output = run_commands(temp_dir.path(), &[&["work", "move", &work_id, "active"]])?;

    assert!(output.contains("error[E0704]"), "output: {output}");
    assert_eq!(fs::read_to_string(work_path)?, original);
    Ok(())
}

#[test]
fn test_move_reopen_does_not_mutate_existing_loop_files() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    setup_active_work_item_with_criteria(temp_dir.path(), &date, "Task", "add: Done")?;
    run_commands(
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
            &["work", "move", &work_id, "done"],
        ],
    )?;

    let loop_dir = temp_dir.path().join(".govctl/loops/LOOP-2026-01-01-001");
    fs::create_dir_all(loop_dir.join("rounds"))?;
    let state_path = loop_dir.join("state.toml");
    let round_path = loop_dir.join("rounds/round-001.toml");
    let state = format!("work = [\"{work_id}\"]\nstatus = \"done\"\n");
    let round = format!("work_item = \"{work_id}\"\nstatus = \"done\"\n");
    fs::write(&state_path, &state)?;
    fs::write(&round_path, &round)?;

    run_commands(temp_dir.path(), &[&["work", "move", &work_id, "active"]])?;

    assert_eq!(fs::read_to_string(state_path)?, state);
    assert_eq!(fs::read_to_string(round_path)?, round);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_move_write_failure_preserves_work_item_bytes() -> common::TestResult {
    use std::os::unix::fs::PermissionsExt;

    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);
    run_commands(temp_dir.path(), &[&["work", "new", "Atomic move"]])?;

    let work_dir = temp_dir.path().join("gov/work");
    let work_path = fs::read_dir(&work_dir)?
        .next()
        .ok_or("missing work item")??
        .path();
    let original = fs::read(&work_path)?;
    let original_permissions = fs::metadata(&work_dir)?.permissions();
    let mut unwritable = original_permissions.clone();
    unwritable.set_mode(original_permissions.mode() & !0o222);
    fs::set_permissions(&work_dir, unwritable)?;

    let output = run_commands(temp_dir.path(), &[&["work", "move", &work_id, "active"]]);
    fs::set_permissions(&work_dir, original_permissions)?;
    let output = output?;

    assert!(output.contains("error[E0901]"), "output: {output}");
    assert_eq!(fs::read(work_path)?, original);
    Ok(())
}
