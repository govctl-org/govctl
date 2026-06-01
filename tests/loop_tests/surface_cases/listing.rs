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
