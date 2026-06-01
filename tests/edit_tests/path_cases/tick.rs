#[test]
fn test_tick_rejects_nested_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Nested Tick"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "ac[0].text",
                "Criterion 1",
                "-s",
                "done",
            ],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_edit_tick_updates_alternative_root() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Tick Root Test"],
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--tick",
                "accepted",
                "--at",
                "0",
            ],
            &["adr", "get", "ADR-0001", "alternatives"],
        ],
    )?;
    assert!(
        output.contains("Marked 'Option A' as accepted"),
        "output: {}",
        output
    );
    assert!(output.contains("[accepted] Option A"), "output: {}", output);
    Ok(())
}

#[test]
fn test_adr_edit_tick_updates_indexed_alternative_item() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Indexed Tick Test"],
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &["adr", "edit", "ADR-0001", "alt[0]", "--tick", "accepted"],
            &["adr", "get", "ADR-0001", "alternatives[0].status"],
        ],
    )?;
    assert!(
        output.contains("Marked 'Option A' as accepted"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl adr get ADR-0001 alternatives[0].status\naccepted"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_adr_edit_tick_rejects_work_item_status_names() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Invalid Tick Test"],
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--tick",
                "done",
                "--at",
                "0",
            ],
        ],
    )?;
    assert!(output.contains("error[E0820]"), "output: {}", output);
    assert!(
        output.contains("ADR tick status must be one of: accepted, considered, rejected"),
        "output: {}",
        output
    );
    Ok(())
}
