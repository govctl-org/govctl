use super::*;

#[test]
fn test_loop_run_targets_work_item_without_selecting_unrelated_work() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_id = format!("WI-{date}-001");
    let second_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("First"),
            work_new("Second"),
            loop_start_with_id(&loop_id, &[&first_id, &second_id]),
            loop_run_target(&loop_id, &first_id),
        ],
    )?;

    assert!(output.contains(&format!("Targets: {first_id}")), "{output}");
    assert!(
        output.contains(&format!("Opened round 1 for loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_item_status(&state_toml, &first_id)?, "active");
    assert_eq!(loop_item_round_count(&state_toml, &first_id)?, 1);
    assert_eq!(loop_item_status(&state_toml, &second_id)?, "pending");
    assert_eq!(loop_item_round_count(&state_toml, &second_id)?, 0);

    let round_toml = read_round_record(temp_dir.path(), &loop_id, &first_id, 1)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(
        round["round"]["work"].as_array().ok_or("round work")?.len(),
        1
    );
    assert_eq!(round["round"]["work"][0].as_str(), Some(first_id.as_str()));
    Ok(())
}

#[test]
fn test_loop_run_target_selects_ready_transitive_dependency_first() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let dependency_id = format!("WI-{date}-001");
    let root_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Dependency"),
            work_new("Root"),
            work_add_dependency(&root_id, &dependency_id),
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_run_target(&loop_id, &root_id),
        ],
    )?;

    assert!(output.contains(&format!("Targets: {root_id}")), "{output}");
    assert!(
        output.contains(&format!("Opened round 1 for loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_item_status(&state_toml, &dependency_id)?, "active");
    assert_eq!(loop_item_round_count(&state_toml, &dependency_id)?, 1);
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "pending");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 0);
    let round_toml = read_round_record(temp_dir.path(), &loop_id, &dependency_id, 1)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(
        round["round"]["work"][0].as_str(),
        Some(dependency_id.as_str())
    );
    Ok(())
}

#[test]
fn test_loop_run_rejects_mismatched_target_when_closing_open_round() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_id = format!("WI-{date}-001");
    let second_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("First"),
            work_new("Second"),
            loop_start_with_id(&loop_id, &[&first_id, &second_id]),
            loop_run_target(&loop_id, &first_id),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");
    submit_round_summary(
        temp_dir.path(),
        &loop_id,
        1,
        &["worked on first"],
        &["no changes"],
        &["not run"],
        &[],
    )?;

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run_target(&loop_id, &second_id)])?;

    assert!(output.contains("error[E1201]"), "{output}");
    assert!(
        output.contains(&format!(
            "Loop run target selector does not include open round work item: {first_id}"
        )),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("state = \"active\""), "{state_toml}");
    let round_toml = read_round_record(temp_dir.path(), &loop_id, &first_id, 1)?;
    assert!(
        round_toml.contains("status = \"submitted\""),
        "{round_toml}"
    );
    Ok(())
}

#[test]
fn test_loop_run_rejects_target_outside_loop() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let outside_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            loop_start_with_id(&loop_id, &[&root_id]),
            work_new("Outside"),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run_target(&loop_id, &outside_id)])?;

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
            .join(format!(".govctl/loops/{loop_id}/rounds/round-001.toml"))
            .exists(),
        "invalid targeted run should not open any round"
    );
    Ok(())
}

#[test]
fn test_loop_run_rejects_duplicate_targets_before_state_change() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[work_new("Root"), loop_start_with_id(&loop_id, &[&root_id])],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[command(&[
            "loop", "run", &loop_id, "--work", &root_id, "--work", &root_id,
        ])],
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
