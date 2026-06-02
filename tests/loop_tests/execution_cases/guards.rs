use super::*;

#[test]
fn test_loop_run_records_guard_failure_without_completing_work_item() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            work_add_acceptance(&root_id, "add: ready"),
            work_tick_acceptance_done(&root_id, "ready"),
        ],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1")?;
    append_required_guard(temp_dir.path(), &date, "root", "GUARD-FAIL")?;

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[loop_start_with_id(&loop_id, &[&root_id])],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run(&loop_id)])?;

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
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            work_add_acceptance(&root_id, "add: ready"),
            work_tick_acceptance_done(&root_id, "ready"),
        ],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1")?;
    append_required_guard(temp_dir.path(), &date, "root", "GUARD-FAIL")?;

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[loop_start_with_id(&loop_id, &[&root_id])],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run_with_max_rounds(&loop_id, "2")])?;

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

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run_with_max_rounds(&loop_id, "2")])?;

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
