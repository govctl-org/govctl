use super::*;

#[test]
fn test_loop_list_empty_state() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_dynamic_commands(temp_dir.path(), &[loop_list(&[])])?;

    assert!(output.contains("│ ID"), "{output}");
    assert!(output.contains("State"), "{output}");
    assert!(!output.contains("LOOP-"), "{output}");
    assert!(output.contains("exit: 0"), "{output}");
    Ok(())
}

#[test]
fn test_loop_list_plain_and_json_are_stable() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_root = format!("WI-{date}-001");
    let second_root = format!("WI-{date}-002");
    let first_loop = loop_id(&date, 1);
    let second_loop = loop_id(&date, 2);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("First"),
            work_new("Second"),
            loop_start_with_id(&second_loop, &[&second_root]),
            loop_start_with_id(&first_loop, &[&first_root]),
            loop_list(&["-o", "plain"]),
            loop_list(&["-o", "json"]),
        ],
    )?;

    let first_plain = format!("{first_loop}\tpending\tfresh\t{first_root}\t1\t0\tstart");
    let second_plain = format!("{second_loop}\tpending\tfresh\t{second_root}\t1\t0\tstart");
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
    assert_eq!(loops[0]["plan"], "fresh");
    assert_eq!(loops[0]["work"][0], first_root);
    assert_eq!(loops[0]["items"], 1);
    assert_eq!(loops[0]["rounds"], 0);
    assert_eq!(loops[0]["next_action"], "start");
    assert_eq!(loops[1]["id"], second_loop);
    Ok(())
}

#[test]
fn test_loop_list_rounds_are_aggregate_item_round_counts() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_root = format!("WI-{date}-001");
    let second_root = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("First"),
            work_new("Second"),
            loop_start_with_id(&loop_id, &[&first_root, &second_root]),
            loop_run(&loop_id),
            loop_list(&["-o", "json"]),
        ],
    )?;

    let json_start = output.find("[\n").ok_or("missing JSON list output")?;
    let json_end = output[json_start..]
        .find("\nexit:")
        .ok_or("missing JSON command terminator")?
        + json_start;
    let loops: serde_json::Value = serde_json::from_str(&output[json_start..json_end])?;
    assert_eq!(loops[0]["rounds"], 2, "{output}");
    assert_eq!(loops[0]["next_action"], "write_summary", "{output}");

    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert!(state_toml.contains("current_round = 1"), "{state_toml}");
    Ok(())
}

#[test]
fn test_loop_list_filters_resumable_aliases_and_limit() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let pending_root = format!("WI-{date}-001");
    let paused_root = format!("WI-{date}-002");
    let completed_root = format!("WI-{date}-003");
    let pending_loop = loop_id(&date, 1);
    let paused_loop = loop_id(&date, 2);
    let completed_loop = loop_id(&date, 3);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Pending"),
            loop_start_with_id(&pending_loop, &[&pending_root]),
            work_new("Paused"),
            work_add_acceptance(&paused_root, "add: waiting"),
            loop_start_with_id(&paused_loop, &[&paused_root]),
            loop_run_with_max_rounds(&paused_loop, "2"),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");
    submit_round_summary(
        temp_dir.path(),
        &paused_loop,
        1,
        &["attempted paused work"],
        &["no changes"],
        &[],
        &["blocked"],
    )?;
    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            loop_run_with_max_rounds(&paused_loop, "2"),
            work_new_active("Completed"),
            work_add_acceptance(&completed_root, "add: ready"),
            work_tick_acceptance_done(&completed_root, "ready"),
            loop_start_with_id(&completed_loop, &[&completed_root]),
            loop_run(&completed_loop),
            work_move_done(&completed_root),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");
    submit_round_summary(
        temp_dir.path(),
        &completed_loop,
        1,
        &["completed work"],
        &["gov/work"],
        &["govctl work move succeeded"],
        &[],
    )?;
    let setup_output = run_dynamic_commands(temp_dir.path(), &[loop_run(&completed_loop)])?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let open_output = run_dynamic_commands(temp_dir.path(), &[loop_list(&["open", "-o", "json"])])?;
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

    let paused_output =
        run_dynamic_commands(temp_dir.path(), &[loop_list(&["paused", "-o", "plain"])])?;
    assert!(
        paused_output.contains(&format!(
            "{paused_loop}\tpaused\tfresh\t{paused_root}\t1\t1\tresolve_blocker"
        )),
        "{paused_output}"
    );
    assert!(!paused_output.contains(&pending_loop), "{paused_output}");
    assert!(!paused_output.contains(&completed_loop), "{paused_output}");

    let limited_output = run_dynamic_commands(
        temp_dir.path(),
        &[loop_list(&["resumable", "-n", "1", "-o", "plain"])],
    )?;
    assert!(
        limited_output.contains(&format!(
            "{pending_loop}\tpending\tfresh\t{pending_root}\t1\t0\tstart"
        )),
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
    let (temp_dir, date) = init_project_with_date()?;
    let loop_id = loop_id(&date, 1);
    fs::create_dir_all(temp_dir.path().join(format!(".govctl/loops/{loop_id}")))?;

    let output = run_dynamic_commands(temp_dir.path(), &[loop_list(&[])])?;

    assert!(output.contains("error[E1202]"), "{output}");
    assert!(output.contains("Failed to read loop state"), "{output}");
    assert!(output.contains(&loop_id), "{output}");
    Ok(())
}

#[test]
fn test_loop_show_and_list_report_stale_dependency_plan() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let dependency_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            work_new("Dependency"),
            loop_start_with_id(&loop_id, &[&root_id]),
            work_add_dependency(&root_id, &dependency_id),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let show_output = run_dynamic_commands(temp_dir.path(), &[loop_show(&loop_id)])?;
    assert!(show_output.contains("Plan status: stale"), "{show_output}");

    let plain_output = run_dynamic_commands(temp_dir.path(), &[loop_list(&["-o", "plain"])])?;
    assert!(
        plain_output.contains(&format!(
            "{loop_id}\tpending\tstale\t{root_id}\t1\t0\tstart"
        )),
        "{plain_output}"
    );

    let json_output = run_dynamic_commands(temp_dir.path(), &[loop_list(&["-o", "json"])])?;
    let json_start = json_output.find("[\n").ok_or("missing JSON list output")?;
    let json_end = json_output[json_start..]
        .find("\nexit:")
        .ok_or("missing JSON command terminator")?
        + json_start;
    let loops: serde_json::Value = serde_json::from_str(&json_output[json_start..json_end])?;
    assert_eq!(loops[0]["plan"], "stale", "{json_output}");
    Ok(())
}
