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
