//! Tests for loop command surface.

mod common;

use common::{init_project, run_dynamic_commands};
use std::fs;
use std::path::Path;

fn loop_id(date: &str, sequence: u32) -> String {
    format!("LOOP-{date}-{sequence:03}")
}

#[test]
fn test_loop_start_show_and_resume_by_root_set() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let dependency_id = format!("WI-{date}-001");
    let root_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

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
                loop_id.clone(),
                root_id.clone(),
            ],
            vec!["loop".into(), "show".into(), loop_id.clone()],
            vec!["loop".into(), "resume".into(), root_id.clone()],
        ],
    )?;

    assert!(
        output.contains(&format!("Started loop {loop_id}")),
        "{output}"
    );
    assert!(output.contains(&format!("Loop {loop_id}")), "{output}");
    assert!(output.contains(&format!("Roots: {root_id}")), "{output}");
    assert!(output.contains(&format!("1. {dependency_id}")), "{output}");
    assert!(output.contains(&format!("2. {root_id}")), "{output}");
    assert!(
        output.contains(&format!("depends_on={dependency_id}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Resumed loop {loop_id}")),
        "{output}"
    );

    let state_path = temp_dir
        .path()
        .join(format!(".govctl/loops/{loop_id}/state.toml"));
    let state_toml = fs::read_to_string(&state_path)?;
    assert!(state_toml.contains(&format!("id = \"{loop_id}\"")));
    assert!(state_toml.contains(&format!("root_work_items = [\"{root_id}\"]")));
    assert!(!state_toml.contains("journal"));
    Ok(())
}

#[test]
fn test_loop_start_generates_canonical_daily_sequence_ids() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let first_root = format!("WI-{date}-001");
    let second_root = format!("WI-{date}-002");
    let first_loop = loop_id(&date, 1);
    let second_loop = loop_id(&date, 2);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "First".into()],
            vec!["loop".into(), "start".into(), first_root.clone()],
            vec!["work".into(), "new".into(), "Second".into()],
            vec!["loop".into(), "start".into(), second_root.clone()],
        ],
    )?;

    assert!(
        output.contains(&format!("Started loop {first_loop}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Started loop {second_loop}")),
        "{output}"
    );
    assert!(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{first_loop}/state.toml"))
            .exists()
    );
    assert!(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{second_loop}/state.toml"))
            .exists()
    );
    Ok(())
}

#[test]
fn test_loop_start_rejects_plain_text_loop_id() -> common::TestResult {
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
                "loop-test".into(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(output.contains("error[E1204]"), "{output}");
    assert!(output.contains("LOOP-YYYY-MM-DD-NNN"), "{output}");
    assert!(
        !temp_dir
            .path()
            .join(".govctl/loops/loop-test/state.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn test_loop_schemas_reject_invalid_calendar_dates() -> common::TestResult {
    let temp_dir = init_project()?;
    let invalid_state = r#"
[loop]
id = "LOOP-2026-02-31-001"
state = "pending"
root_work_items = ["WI-2026-02-28-001"]
work_items = ["WI-2026-02-28-001"]

[dependencies]
WI-2026-02-28-001 = []

[items.WI-2026-02-28-001]
status = "pending"
round_count = 0
"#;
    let invalid_round = r#"
loop_id = "LOOP-2026-02-31-001"
work_item_id = "WI-2026-02-28-001"
round_number = 1
max_rounds = 1
item_status_before = "pending"
item_status_after = "active"
work_status_before = "queue"
work_status_after = "active"
action = "evaluated acceptance criteria"
outcome = "active"
"#;

    assert_schema_rejects(
        temp_dir.path(),
        "loop-state.schema.json",
        invalid_state,
        "invalid loop state date should fail schema validation",
    )?;
    assert_schema_rejects(
        temp_dir.path(),
        "loop-round.schema.json",
        invalid_round,
        "invalid loop round date should fail schema validation",
    )?;
    Ok(())
}

#[test]
fn test_loop_start_reuses_existing_loop_id() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Started loop {loop_id}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Reused loop {loop_id}")),
        "{output}"
    );
    Ok(())
}

#[test]
fn test_loop_start_dry_run_previews_state_without_writing() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
                "--dry-run".into(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Would create dir: .govctl/loops/{loop_id}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Would write: .govctl/loops/{loop_id}/state.toml")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Would start loop {loop_id}")),
        "{output}"
    );
    assert!(
        !temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml"))
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

#[test]
fn test_loop_run_completes_ready_work_item() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "add: ready".into(),
            ],
            vec![
                "work".into(),
                "tick".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "ready".into(),
                "-s".into(),
                "done".into(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Running loop {loop_id}")),
        "{output}"
    );
    assert!(output.contains("Max rounds: 1"), "{output}");
    assert!(
        output.contains(&format!("Completed loop {loop_id}")),
        "{output}"
    );

    let work_toml = fs::read_to_string(temp_dir.path().join(format!("gov/work/{date}-root.toml")))?;
    assert!(work_toml.contains("status = \"done\""), "{work_toml}");

    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    validate_toml_against_schema(temp_dir.path(), "loop-state.schema.json", &state_toml)?;
    assert!(state_toml.contains("state = \"completed\""), "{state_toml}");
    assert!(state_toml.contains("status = \"done\""), "{state_toml}");
    assert!(state_toml.contains("round_count = 1"), "{state_toml}");
    assert!(!state_toml.contains("journal"), "{state_toml}");

    let round_toml = read_round_record(temp_dir.path(), &loop_id, &root_id, 1)?;
    validate_toml_against_schema(temp_dir.path(), "loop-round.schema.json", &round_toml)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(toml_string(&round, "loop_id")?, loop_id);
    assert_eq!(toml_string(&round, "work_item_id")?, root_id);
    assert_eq!(toml_int(&round, "round_number")?, 1);
    assert_eq!(toml_int(&round, "max_rounds")?, 1);
    assert_eq!(toml_string(&round, "item_status_before")?, "pending");
    assert_eq!(toml_string(&round, "item_status_after")?, "done");
    assert_eq!(toml_string(&round, "work_status_before")?, "queue");
    assert_eq!(toml_string(&round, "work_status_after")?, "done");
    assert_eq!(toml_string(&round, "outcome")?, "done");
    assert!(
        toml_string(&round, "action")?.contains("acceptance criteria"),
        "{round_toml}"
    );
    assert!(!round_toml.contains("journal"), "{round_toml}");
    Ok(())
}

#[test]
fn test_loop_run_marks_failed_and_blocks_dependents() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let dependency_id = format!("WI-{date}-001");
    let root_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Dependency".into()],
            vec![
                "work".into(),
                "add".into(),
                dependency_id.clone(),
                "acceptance_criteria".into(),
                "add: unfinished".into(),
            ],
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "add: ready".into(),
            ],
            vec![
                "work".into(),
                "tick".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "ready".into(),
                "-s".into(),
                "done".into(),
            ],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "depends_on".into(),
                dependency_id.clone(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Failed loop {loop_id}")),
        "{output}"
    );
    assert!(output.contains("error[E1210]"), "{output}");

    let dependency_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!("gov/work/{date}-dependency.toml")),
    )?;
    assert!(
        dependency_toml.contains("status = \"active\""),
        "{dependency_toml}"
    );
    let root_toml = fs::read_to_string(temp_dir.path().join(format!("gov/work/{date}-root.toml")))?;
    assert!(root_toml.contains("status = \"queue\""), "{root_toml}");

    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"failed\""), "{state_toml}");
    assert_eq!(loop_item_status(&state_toml, &dependency_id)?, "failed");
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "blocked");

    let dependency_round = read_round_record(temp_dir.path(), &loop_id, &dependency_id, 1)?;
    let dependency_round: toml::Value = toml::from_str(&dependency_round)?;
    assert_eq!(toml_string(&dependency_round, "outcome")?, "failed");
    assert!(
        toml_string(&dependency_round, "reason")?.contains("pending acceptance criteria"),
        "{dependency_round:?}"
    );
    assert!(
        !temp_dir
            .path()
            .join(format!(
                ".govctl/loops/{loop_id}/rounds/{root_id}/round-001.toml"
            ))
            .exists(),
        "blocked dependent should not execute a round"
    );
    Ok(())
}

#[test]
fn test_loop_run_records_guard_failure_without_completing_work_item() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "add: ready".into(),
            ],
            vec![
                "work".into(),
                "tick".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "ready".into(),
                "-s".into(),
                "done".into(),
            ],
        ],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1")?;
    append_required_guard(temp_dir.path(), &date, "root", "GUARD-FAIL")?;

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "run".into(),
            "--id".into(),
            loop_id.clone(),
            root_id.clone(),
        ]],
    )?;

    assert!(output.contains("FAIL GUARD-FAIL"), "{output}");
    assert!(output.contains("error[E1210]"), "{output}");

    let work_toml = fs::read_to_string(temp_dir.path().join(format!("gov/work/{date}-root.toml")))?;
    assert!(work_toml.contains("status = \"active\""), "{work_toml}");

    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"failed\""), "{state_toml}");
    assert!(state_toml.contains("status = \"failed\""), "{state_toml}");
    assert!(state_toml.contains("round_count = 1"), "{state_toml}");
    Ok(())
}

#[test]
fn test_loop_run_guard_failure_can_pause_until_max_rounds() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "add: ready".into(),
            ],
            vec![
                "work".into(),
                "tick".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "ready".into(),
                "-s".into(),
                "done".into(),
            ],
        ],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1")?;
    append_required_guard(temp_dir.path(), &date, "root", "GUARD-FAIL")?;

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "run".into(),
            "--id".into(),
            loop_id.clone(),
            "--max-rounds".into(),
            "2".into(),
            root_id.clone(),
        ]],
    )?;

    assert!(output.contains("FAIL GUARD-FAIL"), "{output}");
    assert!(
        output.contains(&format!("Paused loop {loop_id}")),
        "{output}"
    );
    assert!(!output.contains("error[E1210]"), "{output}");
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"paused\""), "{state_toml}");
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "active");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 1);
    let round_toml = read_round_record(temp_dir.path(), &loop_id, &root_id, 1)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(toml_string(&round, "outcome")?, "active");
    assert_eq!(toml_string(&round, "item_status_after")?, "active");
    assert!(
        toml_string(&round, "reason")?.contains("max rounds not reached"),
        "{round_toml}"
    );

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "run".into(),
            "--id".into(),
            loop_id.clone(),
            "--max-rounds".into(),
            "2".into(),
        ]],
    )?;

    assert!(
        output.contains(&format!("Failed loop {loop_id}")),
        "{output}"
    );
    assert!(output.contains("error[E1210]"), "{output}");
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"failed\""), "{state_toml}");
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "failed");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 2);
    let round_toml = read_round_record(temp_dir.path(), &loop_id, &root_id, 2)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(toml_string(&round, "outcome")?, "failed");
    assert_eq!(toml_string(&round, "item_status_after")?, "failed");
    assert!(
        toml_string(&round, "reason")?.contains("failed to complete"),
        "{round_toml}"
    );
    Ok(())
}

#[test]
fn test_loop_run_resumes_paused_loop_without_restarting_done_items() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let dependency_id = format!("WI-{date}-001");
    let root_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Dependency".into()],
            vec![
                "work".into(),
                "add".into(),
                dependency_id.clone(),
                "acceptance_criteria".into(),
                "add: dependency ready".into(),
            ],
            vec![
                "work".into(),
                "tick".into(),
                dependency_id.clone(),
                "acceptance_criteria".into(),
                "dependency ready".into(),
                "-s".into(),
                "done".into(),
            ],
            vec!["work".into(), "new".into(), "Root".into()],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "add: root ready".into(),
            ],
            vec![
                "work".into(),
                "add".into(),
                root_id.clone(),
                "depends_on".into(),
                dependency_id.clone(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                "--id".into(),
                loop_id.clone(),
                "--max-rounds".into(),
                "2".into(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Paused loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"paused\""), "{state_toml}");
    assert!(state_toml.contains("round_count = 1"), "{state_toml}");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec![
                "work".into(),
                "tick".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "root ready".into(),
                "-s".into(),
                "done".into(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                "--max-rounds".into(),
                "2".into(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Completed loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"completed\""), "{state_toml}");
    assert_eq!(
        state_toml.matches("round_count = 1").count(),
        1,
        "{state_toml}"
    );
    assert!(state_toml.contains("round_count = 2"), "{state_toml}");
    Ok(())
}

#[test]
fn test_loop_scope_add_remove_and_replan_preserve_current_state() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let original_id = format!("WI-{date}-001");
    let new_dependency_id = format!("WI-{date}-002");
    let new_root_id = format!("WI-{date}-003");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Original".into()],
            vec![
                "work".into(),
                "add".into(),
                original_id.clone(),
                "acceptance_criteria".into(),
                "add: unfinished".into(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                original_id.clone(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                "--id".into(),
                loop_id.clone(),
                "--max-rounds".into(),
                "2".into(),
            ],
            vec!["work".into(), "new".into(), "Dependency".into()],
            vec!["work".into(), "new".into(), "New root".into()],
            vec![
                "work".into(),
                "add".into(),
                new_root_id.clone(),
                "depends_on".into(),
                new_dependency_id.clone(),
            ],
            vec![
                "loop".into(),
                "add".into(),
                "--id".into(),
                loop_id.clone(),
                new_root_id.clone(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Updated loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_item_round_count(&state_toml, &original_id)?, 1);
    assert_eq!(loop_item_status(&state_toml, &original_id)?, "active");
    let state: toml::Value = toml::from_str(&state_toml)?;
    assert_eq!(
        loop_roots(&state)?,
        vec![original_id.clone(), new_root_id.clone()]
    );
    assert_eq!(
        loop_work_items(&state)?,
        vec![
            original_id.clone(),
            new_dependency_id.clone(),
            new_root_id.clone()
        ]
    );

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec![
                "loop".into(),
                "remove".into(),
                "--id".into(),
                loop_id.clone(),
                original_id.clone(),
            ],
            vec![
                "work".into(),
                "remove".into(),
                new_root_id.clone(),
                "depends_on".into(),
                new_dependency_id.clone(),
            ],
            vec![
                "loop".into(),
                "replan".into(),
                "--id".into(),
                loop_id.clone(),
            ],
        ],
    )?;

    assert!(
        output.contains(&format!("Replanned loop {loop_id}")),
        "{output}"
    );

    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    let state: toml::Value = toml::from_str(&state_toml)?;
    assert_eq!(loop_roots(&state)?, vec![new_root_id.clone()]);
    assert_eq!(loop_work_items(&state)?, vec![new_root_id.clone()]);
    assert!(
        loop_item_table(&state, &original_id).is_err(),
        "removed root should no longer have current item state: {state_toml}"
    );
    assert!(
        loop_item_table(&state, &new_dependency_id).is_err(),
        "replan should remove dependencies no longer needed: {state_toml}"
    );
    Ok(())
}

fn write_guard(dir: &Path, guard_id: &str, command: &str) -> common::TestResult {
    let path = dir
        .join("gov/guard")
        .join(format!("{}.toml", guard_id.to_lowercase()));
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\ntimeout_secs = 300\n"
    );
    fs::write(path, content)?;
    Ok(())
}

fn append_required_guard(dir: &Path, date: &str, slug: &str, guard_id: &str) -> common::TestResult {
    let path = dir.join(format!("gov/work/{date}-{slug}.toml"));
    let mut content = fs::read_to_string(&path)?;
    content.push_str(&format!(
        "\n[verification]\nrequired_guards = [\"{guard_id}\"]\n"
    ));
    fs::write(path, content)?;
    Ok(())
}

fn read_round_record(
    dir: &Path,
    loop_id: &str,
    work_id: &str,
    round: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(dir.join(format!(
        ".govctl/loops/{loop_id}/rounds/{work_id}/round-{round:03}.toml"
    )))?)
}

fn validate_toml_against_schema(
    dir: &Path,
    schema_filename: &str,
    toml_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema_text = fs::read_to_string(dir.join("gov/schema").join(schema_filename))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;
    let compiled = jsonschema::validator_for(&schema)?;
    let toml_value: toml::Value = toml::from_str(toml_text)?;
    let json_value = serde_json::to_value(toml_value)?;
    let errors = compiled
        .iter_errors(&json_value)
        .map(|err| err.to_string())
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "{schema_filename} validation errors: {errors:#?}"
    );
    Ok(())
}

fn assert_schema_rejects(
    dir: &Path,
    schema_filename: &str,
    toml_text: &str,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema_text = fs::read_to_string(dir.join("gov/schema").join(schema_filename))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;
    let compiled = jsonschema::validator_for(&schema)?;
    let toml_value: toml::Value = toml::from_str(toml_text)?;
    let json_value = serde_json::to_value(toml_value)?;
    let errors = compiled
        .iter_errors(&json_value)
        .map(|err| err.to_string())
        .collect::<Vec<_>>();
    assert!(!errors.is_empty(), "{context}");
    Ok(())
}

fn toml_string(value: &toml::Value, key: &str) -> Result<String, Box<dyn std::error::Error>> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field: {key}").into())
}

fn toml_int(value: &toml::Value, key: &str) -> Result<i64, Box<dyn std::error::Error>> {
    value
        .get(key)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| format!("missing integer field: {key}").into())
}

fn loop_item_status(state_toml: &str, work_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let state: toml::Value = toml::from_str(state_toml)?;
    Ok(loop_item_table(&state, work_id)?
        .get("status")
        .and_then(toml::Value::as_str)
        .ok_or("missing loop item status")?
        .to_string())
}

fn loop_item_round_count(
    state_toml: &str,
    work_id: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let state: toml::Value = toml::from_str(state_toml)?;
    loop_item_table(&state, work_id)?
        .get("round_count")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| "missing loop item round_count".into())
}

fn loop_roots(value: &toml::Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    toml_string_array(value, &["loop", "root_work_items"])
}

fn loop_work_items(value: &toml::Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    toml_string_array(value, &["loop", "work_items"])
}

fn toml_string_array(
    value: &toml::Value,
    path: &[&str],
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut current = value;
    for segment in path {
        current = current
            .get(*segment)
            .ok_or_else(|| format!("missing TOML segment: {segment}"))?;
    }
    current
        .as_array()
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            format!("missing array at path: {}", path.join(".")).into()
        })?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("non-string value at path: {}", path.join(".")).into())
        })
        .collect()
}

fn loop_item_table<'a>(
    state: &'a toml::Value,
    work_id: &str,
) -> Result<&'a toml::value::Table, Box<dyn std::error::Error>> {
    state
        .get("items")
        .and_then(toml::Value::as_table)
        .and_then(|items| items.get(work_id))
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing loop item table for {work_id}").into())
}
