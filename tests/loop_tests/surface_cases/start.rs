use super::*;

#[test]
fn test_loop_start_show_and_resume_by_loop_id() -> common::TestResult {
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
            loop_show(&loop_id),
            loop_resume(&loop_id),
        ],
    )?;

    assert!(
        output.contains(&format!("Started loop {loop_id}")),
        "{output}"
    );
    assert!(output.contains(&format!("Loop {loop_id}")), "{output}");
    assert!(output.contains(&format!("Work: {root_id}")), "{output}");
    assert!(output.contains(&format!("1. {dependency_id}")), "{output}");
    assert!(output.contains(&format!("2. {root_id}")), "{output}");
    assert!(
        output.contains(&format!("depends_on={dependency_id}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Resumed loop {loop_id}")),
        "{output}"
    );

    let state_path = temp_dir
        .path()
        .join(format!(".govctl/loops/{loop_id}/state.toml"));
    let state_toml = fs::read_to_string(&state_path)?;
    assert!(state_toml.contains(&format!("id = \"{loop_id}\"")));
    assert!(state_toml.contains(&format!("work = [\"{root_id}\"]")));
    assert!(!state_toml.contains("root_work_items"));
    assert!(!state_toml.contains("journal"));
    Ok(())
}

#[test]
fn test_loop_start_generates_canonical_daily_sequence_ids() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_root = format!("WI-{date}-001");
    let second_root = format!("WI-{date}-002");
    let first_loop = loop_id(&date, 1);
    let second_loop = loop_id(&date, 2);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("First"),
            loop_start(&[&first_root]),
            work_new("Second"),
            loop_start(&[&second_root]),
        ],
    )?;

    assert!(
        output.contains(&format!("Started loop {first_loop}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Started loop {second_loop}")),
        "{output}"
    );
    assert!(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{first_loop}/state.toml"))
            .exists()
    );
    assert!(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{second_loop}/state.toml"))
            .exists()
    );
    Ok(())
}

#[test]
fn test_loop_start_reuses_existing_loop_id() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            loop_start_with_id(&loop_id, &[&root_id]),
            loop_start_with_id(&loop_id, &[&root_id]),
        ],
    )?;

    assert!(
        output.contains(&format!("Started loop {loop_id}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Reused loop {loop_id}")),
        "{output}"
    );
    Ok(())
}

#[test]
fn test_loop_start_dry_run_previews_state_without_writing() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            loop_start_with_id_dry_run(&loop_id, &[&root_id]),
        ],
    )?;

    assert!(
        output.contains(&format!("Would create dir: .govctl/loops/{loop_id}")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Would write: .govctl/loops/{loop_id}/state.toml")),
        "{output}"
    );
    assert!(
        output.contains(&format!("Would start loop {loop_id}")),
        "{output}"
    );
    assert!(
        !temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml"))
            .exists()
    );
    Ok(())
}

#[test]
fn test_loop_resume_missing_loop_id_reports_diagnostic() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(temp_dir.path(), &[loop_resume(&loop_id)])?;

    assert!(output.contains("error[E1202]"), "{output}");
    assert!(output.contains("Failed to read loop state"), "{output}");
    Ok(())
}

#[test]
fn test_loop_resume_completed_loop_reports_terminal_diagnostic() -> common::TestResult {
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
            loop_resume(&loop_id),
        ],
    )?;

    assert!(
        output.contains(&format!("Completed loop {loop_id}")),
        "{output}"
    );
    assert!(output.contains("error[E1210]"), "{output}");
    assert!(
        output.contains(&format!(
            "Cannot resume terminal loop '{loop_id}' in completed state"
        )),
        "{output}"
    );
    Ok(())
}
