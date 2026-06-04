use super::*;

#[test]
fn test_loop_run_does_not_execute_required_guards() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new_active("Root"),
            work_add_acceptance(&root_id, "add: ready"),
        ],
    )?;
    assert!(output.contains("exit: 0"), "{output}");

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1")?;
    append_required_guard(temp_dir.path(), &date, "root", "GUARD-FAIL")?;

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_run(&loop_id),
        ],
    )?;

    assert!(
        output.contains(&format!("Opened round 1 for loop {loop_id}")),
        "{output}"
    );
    assert!(!output.contains("FAIL GUARD-FAIL"), "{output}");
    assert!(!output.contains("error[E1210]"), "{output}");

    let work_toml = fs::read_to_string(temp_dir.path().join(format!("gov/work/{date}-root.toml")))?;
    assert!(work_toml.contains("status = \"active\""), "{work_toml}");
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_item_status(&state_toml, &root_id)?, "active");
    assert_eq!(loop_item_round_count(&state_toml, &root_id)?, 1);
    Ok(())
}

#[test]
fn test_loop_run_closes_round_with_recorded_verification_evidence() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new_active("Root"),
            work_add_acceptance(&root_id, "add: ready"),
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_run(&loop_id),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");
    submit_round_summary(
        temp_dir.path(),
        &loop_id,
        1,
        &["ran work-item guard set"],
        &["no changes"],
        &["FAIL GUARD-FAIL"],
        &[],
    )?;

    let output = run_dynamic_commands(temp_dir.path(), &[loop_run(&loop_id)])?;

    assert!(
        output.contains(&format!("Paused loop {loop_id}")),
        "{output}"
    );
    let round_toml = read_round_record(temp_dir.path(), &loop_id, &root_id, 1)?;
    let round: toml::Value = toml::from_str(&round_toml)?;
    assert_eq!(round["round"]["status"].as_str(), Some("closed"));
    assert_eq!(
        round["summary"]["verification"][0].as_str(),
        Some("FAIL GUARD-FAIL")
    );
    Ok(())
}
