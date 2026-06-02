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
            &["adr", "set", "ADR-0001", "title", "New Title"],
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
            &["adr", "set", "ADR-0001", "status", "accepted"],
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
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &[
                "adr",
                "set",
                "ADR-0001",
                "alternatives[0].status",
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

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "refs", "RFC-0001"],
            &["adr", "get", "ADR-0001", "refs"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
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
                "content.alternatives",
                "--add",
                "Option A",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "content.alternatives[0].pros",
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
                "set",
                "ADR-0001",
                "context",
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
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &["adr", "add", "ADR-0001", "alternatives", "Option B"],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "0",
                "-s",
                "accepted",
            ],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "1",
                "-s",
                "rejected",
            ],
            &["adr", "set", "ADR-0001", "decision", "We decided to do X"],
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
                "set",
                "ADR-0001",
                "consequences",
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
