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
