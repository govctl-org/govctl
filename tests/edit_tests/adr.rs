use super::*;

// ============================================================================
// ADR Field Edit Tests
// ============================================================================

#[test]
fn test_adr_get_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "get", "ADR-0001", "title"],
            &["adr", "get", "ADR-0001", "status"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_set_title() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Original Title"],
            &["adr", "edit", "ADR-0001", "title", "--set", "New Title"],
            &["adr", "list"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_set_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "edit", "ADR-0001", "status", "--set", "accepted"],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl adr accept"), "output: {}", output);
    Ok(())
}

#[test]
fn test_adr_set_alternative_status_field_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--add",
                "Option A",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives[0].status",
                "--set",
                "accepted",
            ],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("tick-owned"), "output: {}", output);
    Ok(())
}

#[test]
fn test_adr_add_ref() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    common::write_minimal_rfc(temp_dir.path(), "RFC-0001", "Referenced RFC")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "edit", "ADR-0001", "refs", "--add", "RFC-0001"],
            &["adr", "get", "ADR-0001", "refs"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_refs_reject_work_item_and_preserve_existing_value() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = work_id(&date, 1);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            command(&["rfc", "new", "Governing RFC"]),
            command(&["adr", "new", "Test Decision"]),
            work_new("Execution Task"),
            command(&["adr", "edit", "ADR-0001", "refs", "--add", &work_id]),
            command(&["adr", "edit", "ADR-0001", "refs", "--add", "RFC-0001"]),
            command(&["adr", "edit", "ADR-0001", "refs[0]", "--set", &work_id]),
            command(&["adr", "get", "ADR-0001", "refs"]),
        ],
    )?;

    assert!(output.contains("error[E0306]"), "output: {}", output);
    assert!(
        !output.contains(&format!("Added '{work_id}' to ADR-0001.refs")),
        "output: {}",
        output
    );
    assert!(
        !output.contains(&format!("Set ADR-0001.refs[0] = {work_id}")),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl adr get ADR-0001 refs\nRFC-0001"),
        "output: {}",
        output
    );
    let adr = std::fs::read_to_string(temp_dir.path().join("gov/adr/ADR-0001-test-decision.toml"))?;
    assert!(!adr.contains(&work_id), "adr.toml: {}", adr);
    Ok(())
}

#[test]
fn test_adr_edit_add_nested_path_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Canonical Edit ADR"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--add",
                "Option A",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives[0].pros",
                "--add",
                "Readable",
            ],
            &["adr", "get", "ADR-0001", "alternatives[0].pros"],
        ],
    )?;

    assert!(
        output.contains("Added 'Option A' to ADR-0001.alternatives"),
        "output: {}",
        output
    );
    assert!(
        output.contains("Added 'Readable' to ADR-0001.alternatives[0].pros"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl adr get ADR-0001 alternatives[0].pros\nReadable"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_adr_set_context() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "context",
                "--set",
                "New context for the decision",
            ],
            &["adr", "show", "ADR-0001"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_set_decision() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            // Implements [[ADR-0042]]: must have alternatives before setting decision
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--add",
                "Option A",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--add",
                "Option B",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives[0]",
                "--tick",
                "accepted",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives[1]",
                "--tick",
                "rejected",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "decision",
                "--set",
                "We decided to do X",
            ],
            &["adr", "show", "ADR-0001"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_set_consequences() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "consequences",
                "--set",
                "Faster reads, but more memory use.",
            ],
            &["adr", "show", "ADR-0001"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_get_nonexistent() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["adr", "get", "ADR-9999", "title"]])?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
