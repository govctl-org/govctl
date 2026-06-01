use crate::common;
use crate::common::loop_helpers::{
    append_required_guard, loop_id, loop_item_round_count, loop_item_status, read_round_record,
    toml_int, toml_string, validate_toml_against_schema, write_guard,
};
use crate::common::{init_project, run_dynamic_commands};
use std::fs;

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
