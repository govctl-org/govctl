use super::*;

#[test]
fn test_loop_run_opens_round_without_mutating_work_item() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            work_add_acceptance(&root_id, "add: ready"),
            work_tick_acceptance_done(&root_id, "ready"),
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_run(&loop_id),
        ],
    )?;

    assert!(
        output.contains(&format!("Running loop {loop_id}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Opened round 1 for loop {loop_id}")),
        "{output}"
    );
    assert!(
        output.contains("Next action: fill summary evidence"),
        "{output}"
    );

    let work_toml = fs::read_to_string(temp_dir.path().join(format!("gov/work/{date}-root.toml")))?;
    assert!(work_toml.contains("status = \"queue\""), "{work_toml}");

    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    validate_toml_against_schema(temp_dir.path(), "loop-state.schema.json", &state_toml)?;
    assert!(state_toml.contains("state = \"active\""), "{state_toml}");
    assert!(state_toml.contains("current_round = 1"), "{state_toml}");
    assert!(
        state_toml.contains("next_action = \"write_summary\""),
        "{state_toml}"
    );
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "active");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 1);
    assert!(state_toml.contains("last_round = 1"), "{state_toml}");
    assert!(!state_toml.contains("journal"), "{state_toml}");

    let round_toml = read_round_record(temp_dir.path(), &loop_id, &root_id, 1)?;
    validate_toml_against_schema(temp_dir.path(), "loop-round.schema.json", &round_toml)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(round["round"]["loop_id"].as_str(), Some(loop_id.as_str()));
    assert_eq!(round["round"]["round_number"].as_integer(), Some(1));
    assert!(round["round"].get("max_rounds").is_none(), "{round_toml}");
    assert_eq!(round["round"]["status"].as_str(), Some("open"));
    assert_eq!(round["round"]["work"][0].as_str(), Some(root_id.as_str()));
    assert!(
        round["summary"]["actions"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{round_toml}"
    );
    assert!(!round_toml.contains("journal"), "{round_toml}");
    Ok(())
}

#[test]
fn test_loop_run_rejects_incomplete_open_round_without_state_change() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_run(&loop_id),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run(&loop_id)])?;

    assert!(output.contains("error[E1210]"), "{output}");
    assert!(
        output.contains("Loop round summary is incomplete"),
        "{output}"
    );
    assert!(
        output.contains(&format!(".govctl/loops/{loop_id}/rounds/round-001.toml")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"active\""), "{state_toml}");
    assert!(
        state_toml.contains("next_action = \"write_summary\""),
        "{state_toml}"
    );
    let round_toml = read_round_record(temp_dir.path(), &loop_id, &root_id, 1)?;
    assert!(round_toml.contains("status = \"open\""), "{round_toml}");
    Ok(())
}

#[test]
fn test_loop_run_closes_submitted_round_and_reflects_done_work() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new_active("Root"),
            work_add_acceptance(&root_id, "add: ready"),
            work_tick_acceptance_done(&root_id, "ready"),
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_run(&loop_id),
            work_move_done(&root_id),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");
    submit_round_summary(
        temp_dir.path(),
        &loop_id,
        1,
        &["implemented root work"],
        &["gov/work"],
        &["govctl work move succeeded"],
        &[],
    )?;

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run(&loop_id)])?;

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
    assert!(
        state_toml.contains("next_action = \"complete\""),
        "{state_toml}"
    );
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "done");

    let round_toml = read_round_record(temp_dir.path(), &loop_id, &root_id, 1)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(round["round"]["status"].as_str(), Some("closed"));
    assert_eq!(
        round["summary"]["actions"][0].as_str(),
        Some("implemented root work")
    );
    Ok(())
}

#[test]
fn test_loop_run_closes_blocked_round_as_paused_then_opens_next_round() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_run(&loop_id),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");
    submit_round_summary(
        temp_dir.path(),
        &loop_id,
        1,
        &["attempted root work"],
        &["no changes"],
        &[],
        &["blocked on missing decision"],
    )?;

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run(&loop_id)])?;

    assert!(
        output.contains(&format!("Paused loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(
        state_toml.contains("next_action = \"resolve_blocker\""),
        "{state_toml}"
    );
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "active");

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run(&loop_id)])?;
    assert!(
        output.contains(&format!("Opened round 2 for loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("current_round = 2"), "{state_toml}");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 2);
    Ok(())
}
