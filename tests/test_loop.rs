//! Tests for loop command surface.

mod common;

use common::loop_helpers::{
    append_required_guard, assert_schema_rejects, loop_id, loop_item_round_count, loop_item_status,
    loop_item_table, loop_roots, loop_work_items, read_round_record, toml_int, toml_string,
    validate_toml_against_schema, write_guard,
};
use common::{init_project, run_dynamic_commands};
use std::fs;

#[test]
fn test_loop_start_show_and_resume_by_loop_id() -> common::TestResult {
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
            vec!["loop".into(), "resume".into(), loop_id.clone()],
        ],
    )?;

    assert!(
        output.contains(&format!("Started loop {loop_id}")),
        "{output}"
    );
    assert!(output.contains(&format!("Loop {loop_id}")), "{output}");
    assert!(output.contains(&format!("Work: {root_id}")), "{output}");
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
fn test_loop_list_empty_state() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_dynamic_commands(temp_dir.path(), &[vec!["loop".into(), "list".into()]])?;

    assert!(output.contains("│ ID"), "{output}");
    assert!(output.contains("State"), "{output}");
    assert!(!output.contains("LOOP-"), "{output}");
    assert!(output.contains("exit: 0"), "{output}");
    Ok(())
}

#[test]
fn test_loop_list_plain_and_json_are_stable() -> common::TestResult {
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
            vec!["work".into(), "new".into(), "Second".into()],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                second_loop.clone(),
                second_root.clone(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                first_loop.clone(),
                first_root.clone(),
            ],
            vec!["loop".into(), "list".into(), "-o".into(), "plain".into()],
            vec!["loop".into(), "list".into(), "-o".into(), "json".into()],
        ],
    )?;

    let first_plain = format!("{first_loop}\tpending\t{first_root}\t1\t0");
    let second_plain = format!("{second_loop}\tpending\t{second_root}\t1\t0");
    assert!(output.contains(&first_plain), "{output}");
    assert!(output.contains(&second_plain), "{output}");
    assert!(
        output.find(&first_plain) < output.find(&second_plain),
        "{output}"
    );

    let json_start = output.find("[\n").ok_or("missing JSON list output")?;
    let json_end = output[json_start..]
        .find("\nexit:")
        .ok_or("missing JSON command terminator")?
        + json_start;
    let loops: serde_json::Value = serde_json::from_str(&output[json_start..json_end])?;
    assert_eq!(
        loops
            .as_array()
            .ok_or("json output should be an array")?
            .len(),
        2
    );
    assert_eq!(loops[0]["id"], first_loop);
    assert_eq!(loops[0]["state"], "pending");
    assert_eq!(loops[0]["work"][0], first_root);
    assert_eq!(loops[0]["resolved_work_items"], 1);
    assert_eq!(loops[0]["rounds"], 0);
    assert_eq!(loops[1]["id"], second_loop);
    Ok(())
}

#[test]
fn test_loop_list_filters_resumable_aliases_and_limit() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let pending_root = format!("WI-{date}-001");
    let paused_root = format!("WI-{date}-002");
    let completed_root = format!("WI-{date}-003");
    let pending_loop = loop_id(&date, 1);
    let paused_loop = loop_id(&date, 2);
    let completed_loop = loop_id(&date, 3);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "Pending".into()],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                pending_loop.clone(),
                pending_root.clone(),
            ],
            vec!["work".into(), "new".into(), "Paused".into()],
            vec![
                "work".into(),
                "add".into(),
                paused_root.clone(),
                "acceptance_criteria".into(),
                "add: waiting".into(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                paused_loop.clone(),
                paused_root.clone(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                paused_loop.clone(),
                "--max-rounds".into(),
                "2".into(),
            ],
            vec!["work".into(), "new".into(), "Completed".into()],
            vec![
                "work".into(),
                "add".into(),
                completed_root.clone(),
                "acceptance_criteria".into(),
                "add: ready".into(),
            ],
            vec![
                "work".into(),
                "tick".into(),
                completed_root.clone(),
                "acceptance_criteria".into(),
                "ready".into(),
                "-s".into(),
                "done".into(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                completed_loop.clone(),
                completed_root.clone(),
            ],
            vec!["loop".into(), "run".into(), completed_loop.clone()],
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let open_output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "list".into(),
            "open".into(),
            "-o".into(),
            "json".into(),
        ]],
    )?;
    let json_start = open_output.find("[\n").ok_or("missing JSON list output")?;
    let json_end = open_output[json_start..]
        .find("\nexit:")
        .ok_or("missing JSON command terminator")?
        + json_start;
    let loops: serde_json::Value = serde_json::from_str(&open_output[json_start..json_end])?;
    let ids = loops
        .as_array()
        .ok_or("json output should be an array")?
        .iter()
        .map(|entry| {
            entry["id"]
                .as_str()
                .ok_or("json loop entry should have string id")
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(ids, vec![pending_loop.as_str(), paused_loop.as_str()]);
    assert_eq!(loops[0]["state"], "pending");
    assert_eq!(loops[1]["state"], "paused");

    let paused_output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "list".into(),
            "paused".into(),
            "-o".into(),
            "plain".into(),
        ]],
    )?;
    assert!(
        paused_output.contains(&format!("{paused_loop}\tpaused\t{paused_root}\t1\t1")),
        "{paused_output}"
    );
    assert!(!paused_output.contains(&pending_loop), "{paused_output}");
    assert!(!paused_output.contains(&completed_loop), "{paused_output}");

    let limited_output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "list".into(),
            "resumable".into(),
            "-n".into(),
            "1".into(),
            "-o".into(),
            "plain".into(),
        ]],
    )?;
    assert!(
        limited_output.contains(&format!("{pending_loop}\tpending\t{pending_root}\t1\t0")),
        "{limited_output}"
    );
    assert!(!limited_output.contains(&paused_loop), "{limited_output}");
    assert!(
        !limited_output.contains(&completed_loop),
        "{limited_output}"
    );
    Ok(())
}

#[test]
fn test_loop_list_reports_invalid_canonical_state() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let loop_id = loop_id(&date, 1);
    fs::create_dir_all(temp_dir.path().join(format!(".govctl/loops/{loop_id}")))?;

    let output = run_dynamic_commands(temp_dir.path(), &[vec!["loop".into(), "list".into()]])?;

    assert!(output.contains("error[E1202]"), "{output}");
    assert!(output.contains("Failed to read loop state"), "{output}");
    assert!(output.contains(&loop_id), "{output}");
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
fn test_loop_resume_missing_loop_id_reports_diagnostic() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec!["loop".into(), "resume".into(), loop_id.clone()]],
    )?;

    assert!(output.contains("error[E1202]"), "{output}");
    assert!(output.contains("Failed to read loop state"), "{output}");
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
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
            vec!["loop".into(), "run".into(), loop_id.clone()],
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
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
            vec!["loop".into(), "run".into(), loop_id.clone()],
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
            "start".into(),
            "--id".into(),
            loop_id.clone(),
            root_id.clone(),
        ]],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec!["loop".into(), "run".into(), loop_id.clone()]],
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
            "start".into(),
            "--id".into(),
            loop_id.clone(),
            root_id.clone(),
        ]],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "run".into(),
            loop_id.clone(),
            "--max-rounds".into(),
            "2".into(),
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
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                loop_id.clone(),
                "--max-rounds".into(),
                "2".into(),
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
                loop_id.clone(),
                "--max-rounds".into(),
                "2".into(),
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
fn test_loop_run_targets_work_item_without_executing_unrelated_work() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let first_id = format!("WI-{date}-001");
    let second_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".into(), "new".into(), "First".into()],
            vec![
                "work".into(),
                "add".into(),
                first_id.clone(),
                "acceptance_criteria".into(),
                "add: first ready".into(),
            ],
            vec![
                "work".into(),
                "tick".into(),
                first_id.clone(),
                "acceptance_criteria".into(),
                "first ready".into(),
                "-s".into(),
                "done".into(),
            ],
            vec!["work".into(), "new".into(), "Second".into()],
            vec![
                "work".into(),
                "add".into(),
                second_id.clone(),
                "acceptance_criteria".into(),
                "add: second pending".into(),
            ],
            vec![
                "loop".into(),
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                first_id.clone(),
                second_id.clone(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                loop_id.clone(),
                "--work".into(),
                first_id.clone(),
            ],
        ],
    )?;

    assert!(output.contains(&format!("Targets: {first_id}")), "{output}");
    assert!(
        output.contains(&format!("Paused loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_item_status(&state_toml, &first_id)?, "done");
    assert_eq!(loop_item_round_count(&state_toml, &first_id)?, 1);
    assert_eq!(loop_item_status(&state_toml, &second_id)?, "pending");
    assert_eq!(loop_item_round_count(&state_toml, &second_id)?, 0);
    assert!(
        !temp_dir
            .path()
            .join(format!(
                ".govctl/loops/{loop_id}/rounds/{second_id}/round-001.toml"
            ))
            .exists(),
        "unrelated work item should not execute a targeted round"
    );
    Ok(())
}

#[test]
fn test_loop_run_target_includes_transitive_dependencies() -> common::TestResult {
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
                "tick".into(),
                root_id.clone(),
                "acceptance_criteria".into(),
                "root ready".into(),
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
                "start".into(),
                "--id".into(),
                loop_id.clone(),
                root_id.clone(),
            ],
            vec![
                "loop".into(),
                "run".into(),
                loop_id.clone(),
                "--work".into(),
                root_id.clone(),
            ],
        ],
    )?;

    assert!(output.contains(&format!("Targets: {root_id}")), "{output}");
    assert!(
        output.contains(&format!("Completed loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_item_status(&state_toml, &dependency_id)?, "done");
    assert_eq!(loop_item_round_count(&state_toml, &dependency_id)?, 1);
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "done");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 1);
    read_round_record(temp_dir.path(), &loop_id, &dependency_id, 1)?;
    read_round_record(temp_dir.path(), &loop_id, &root_id, 1)?;
    Ok(())
}

#[test]
fn test_loop_run_rejects_target_outside_loop() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let outside_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
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
            vec!["work".into(), "new".into(), "Outside".into()],
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "run".into(),
            loop_id.clone(),
            "--work".into(),
            outside_id.clone(),
        ]],
    )?;

    assert!(output.contains("error[E1201]"), "{output}");
    assert!(
        output.contains(&format!(
            "Loop run target '{outside_id}' is not part of loop '{loop_id}'"
        )),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"pending\""), "{state_toml}");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 0);
    assert!(
        !temp_dir
            .path()
            .join(format!(
                ".govctl/loops/{loop_id}/rounds/{root_id}/round-001.toml"
            ))
            .exists(),
        "invalid targeted run should not execute any round"
    );
    Ok(())
}

#[test]
fn test_loop_run_rejects_duplicate_targets_before_state_change() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
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
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "run".into(),
            loop_id.clone(),
            "--work".into(),
            root_id.clone(),
            "--work".into(),
            root_id.clone(),
        ]],
    )?;

    assert!(output.contains("error[E1201]"), "{output}");
    assert!(
        output.contains(&format!("duplicate loop run target work item: {root_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"pending\""), "{state_toml}");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 0);
    Ok(())
}

#[test]
fn test_loop_add_remove_work_field_rejects_unknown_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let root_id = format!("WI-{date}-001");
    let extra_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
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
            vec!["work".into(), "new".into(), "Extra".into()],
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "loop".into(),
            "add".into(),
            loop_id.clone(),
            "work_items".into(),
            extra_id.clone(),
        ]],
    )?;

    assert!(output.contains("error[E0803]"), "{output}");
    assert!(
        output.contains("Unknown loop field: work_items"),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_roots(&toml::from_str(&state_toml)?)?, vec![root_id]);
    Ok(())
}

#[test]
fn test_loop_root_aliases_are_not_supported() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = common::today();
    let loop_id = loop_id(&date, 1);
    let work_id = format!("WI-{date}-001");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec![
                "loop".into(),
                "add-root".into(),
                loop_id.clone(),
                "work".into(),
                work_id.clone(),
            ],
            vec![
                "loop".into(),
                "remove-root".into(),
                loop_id,
                "work".into(),
                work_id,
            ],
        ],
    )?;

    assert!(output.contains("unrecognized subcommand"), "{output}");
    assert!(output.contains("add-root"), "{output}");
    assert!(output.contains("remove-root"), "{output}");
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
                loop_id.clone(),
                "work".into(),
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
                loop_id.clone(),
                "wi".into(),
                original_id.clone(),
            ],
            vec![
                "work".into(),
                "remove".into(),
                new_root_id.clone(),
                "depends_on".into(),
                new_dependency_id.clone(),
            ],
            vec!["loop".into(), "replan".into(), loop_id.clone()],
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
