use crate::common;
use crate::common::loop_helpers::{assert_schema_rejects, loop_id};
use crate::common::{init_project, run_dynamic_commands};
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
    assert!(state_toml.contains(&format!("work = [\"{root_id}\"]")));
    assert!(!state_toml.contains("root_work_items"));
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
    assert_eq!(loops[0]["items"], 1);
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
work = ["WI-2026-02-28-001"]
resolved = ["WI-2026-02-28-001"]

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
    let legacy_state = r#"
[loop]
id = "LOOP-2026-02-28-001"
state = "pending"
root_work_items = ["WI-2026-02-28-001"]
work_items = ["WI-2026-02-28-001"]

[dependencies]
WI-2026-02-28-001 = []

[items.WI-2026-02-28-001]
status = "pending"
round_count = 0
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
    assert_schema_rejects(
        temp_dir.path(),
        "loop-state.schema.json",
        legacy_state,
        "legacy loop state keys should fail schema validation",
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
