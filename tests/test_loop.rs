//! Tests for loop command surface.

mod common;

use common::{init_project, run_dynamic_commands};
use std::fs;

#[test]
fn test_loop_start_show_and_resume_by_root_set() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let dependency_id = format!("WI-{date}-001");
    let root_id = format!("WI-{date}-002");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Dependency".into()],
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "depends_on".into(),
                dependency_id.clone(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                "loop-test".into(),
                root_id.clone(),
            ],
            vec!["loop".into(), "show".into(), "loop-test".into()],
            vec!["loop".into(), "resume".into(), root_id.clone()],
        ],
    )?;

    assert!(output.contains("Started loop loop-test"), "{output}");
    assert!(output.contains("Loop loop-test"), "{output}");
    assert!(output.contains(&format!("Roots: {root_id}")), "{output}");
    assert!(output.contains(&format!("1. {dependency_id}")), "{output}");
    assert!(output.contains(&format!("2. {root_id}")), "{output}");
    assert!(
        output.contains(&format!("depends_on={dependency_id}")),
        "{output}"
    );
    assert!(output.contains("Resumed loop loop-test"), "{output}");

    let state_path = temp_dir.path().join(".govctl/loops/loop-test/state.toml");
    let state_toml = fs::read_to_string(&state_path)?;
    assert!(state_toml.contains("id = \"loop-test\""));
    assert!(state_toml.contains(&format!("root_work_items = [\"{root_id}\"]")));
    assert!(!state_toml.contains("journal"));
    Ok(())
}

#[test]
fn test_loop_start_reuses_existing_loop_id() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                "loop-reuse".into(),
                root_id.clone(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                "loop-reuse".into(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(output.contains("Started loop loop-reuse"), "{output}");
    assert!(output.contains("Reused loop loop-reuse"), "{output}");
    Ok(())
}

#[test]
fn test_loop_start_dry_run_previews_state_without_writing() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                "loop-dry-run".into(),
                root_id.clone(),
                "--dry-run".into(),
            ],
        ],
    )?;

    assert!(
        output.contains("Would create dir: .govctl/loops/loop-dry-run"),
        "{output}"
    );
    assert!(
        output.contains("Would write: .govctl/loops/loop-dry-run/state.toml"),
        "{output}"
    );
    assert!(output.contains("Would start loop loop-dry-run"), "{output}");
    assert!(
        !temp_dir
            .path()
            .join(".govctl/loops/loop-dry-run/state.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn test_loop_resume_missing_root_set_reports_diagnostic() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec!["loop".into(), "resume".into(), root_id.clone()],
        ],
    )?;

    assert!(output.contains("error[E1207]"), "{output}");
    assert!(
        output.contains("No matching non-terminal loop state"),
        "{output}"
    );
    Ok(())
}
