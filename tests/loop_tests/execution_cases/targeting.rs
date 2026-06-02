use super::*;

fn command(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

fn work_new(title: &str) -> Vec<String> {
    command(&["work", "new", title])
}

fn add_acceptance(work_id: &str, text: &str) -> Vec<String> {
    command(&["work", "add", work_id, "acceptance_criteria", text])
}

fn tick_acceptance_done(work_id: &str, pattern: &str) -> Vec<String> {
    command(&[
        "work",
        "tick",
        work_id,
        "acceptance_criteria",
        pattern,
        "-s",
        "done",
    ])
}

fn add_dependency(work_id: &str, dependency_id: &str) -> Vec<String> {
    command(&["work", "add", work_id, "depends_on", dependency_id])
}

fn loop_start(loop_id: &str, work_ids: &[&str]) -> Vec<String> {
    let mut cmd = command(&["loop", "start", "--id", loop_id]);
    cmd.extend(work_ids.iter().map(|work_id| (*work_id).to_string()));
    cmd
}

fn loop_run_target(loop_id: &str, work_id: &str) -> Vec<String> {
    command(&["loop", "run", loop_id, "--work", work_id])
}

#[test]
fn test_loop_run_targets_work_item_without_executing_unrelated_work() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_id = format!("WI-{date}-001");
    let second_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("First"),
            add_acceptance(&first_id, "add: first ready"),
            tick_acceptance_done(&first_id, "first ready"),
            work_new("Second"),
            add_acceptance(&second_id, "add: second pending"),
            loop_start(&loop_id, &[&first_id, &second_id]),
            loop_run_target(&loop_id, &first_id),
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
    let (temp_dir, date) = init_project_with_date()?;
    let dependency_id = format!("WI-{date}-001");
    let root_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Dependency"),
            add_acceptance(&dependency_id, "add: dependency ready"),
            tick_acceptance_done(&dependency_id, "dependency ready"),
            work_new("Root"),
            add_acceptance(&root_id, "add: root ready"),
            tick_acceptance_done(&root_id, "root ready"),
            add_dependency(&root_id, &dependency_id),
            loop_start(&loop_id, &[&root_id]),
            loop_run_target(&loop_id, &root_id),
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
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let outside_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            loop_start(&loop_id, &[&root_id]),
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
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[work_new("Root"), loop_start(&loop_id, &[&root_id])],
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
