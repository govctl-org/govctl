use super::*;

// ============================================================================
// ADR Accept/Reject Tests
// ============================================================================

#[test]
fn test_accept_proposed_adr() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            // Implements [[ADR-0042]]: must have alternatives before accepting
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
            &["adr", "list"],
            &["adr", "accept", "ADR-0001"],
            &["adr", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_reject_proposed_adr() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Bad Decision"],
            &["adr", "reject", "ADR-0001"],
            &["adr", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_accept_already_accepted_fails() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            // Implements [[ADR-0042]]: must have alternatives before accepting
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
            &["adr", "accept", "ADR-0001"],
            &["adr", "accept", "ADR-0001"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_supersede_adr() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Old Decision"],
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
            &["adr", "accept", "ADR-0001"],
            &["adr", "new", "New Decision"],
            &[
                "adr",
                "supersede",
                "ADR-0001",
                "--by",
                "ADR-0002",
                "--force",
            ],
            &["adr", "get", "ADR-0001", "status"],
            &["adr", "get", "ADR-0001", "superseded_by"],
        ],
    )?;

    assert!(
        output.contains("Superseded ADR: ADR-0001"),
        "output: {output}"
    );
    assert!(output.contains("Replaced by: ADR-0002"), "output: {output}");
    assert!(output.contains("$ govctl adr get ADR-0001 status\nsuperseded"));
    assert!(output.contains("$ govctl adr get ADR-0001 superseded_by\nADR-0002"));
    Ok(())
}

#[test]
fn test_supersede_adr_propagates_replacement_lookup_errors() -> common::TestResult {
    let temp_dir = init_project()?;
    let adr_dir = temp_dir.path().join("gov/adr");
    std::fs::remove_dir_all(&adr_dir)?;
    std::fs::write(&adr_dir, "not a directory")?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "adr",
            "supersede",
            "ADR-0001",
            "--by",
            "ADR-0002",
            "--force",
        ]],
    )?;
    assert!(output.contains("error[E0901]"), "{output}");
    assert!(!output.contains("Replacement ADR not found"), "{output}");
    Ok(())
}

#[test]
fn test_accept_adr_propagates_lookup_errors() -> common::TestResult {
    let temp_dir = init_project()?;
    let adr_dir = temp_dir.path().join("gov/adr");
    std::fs::remove_dir_all(&adr_dir)?;
    std::fs::write(&adr_dir, "not a directory")?;

    let output = run_commands(temp_dir.path(), &[&["adr", "accept", "ADR-0001"]])?;
    assert!(output.contains("error[E0901]"), "{output}");
    assert!(!output.contains("ADR not found"), "{output}");
    Ok(())
}

#[test]
fn test_supersede_adr_rejects_missing_replacement() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Old Decision"],
            &[
                "adr",
                "supersede",
                "ADR-0001",
                "--by",
                "ADR-9999",
                "--force",
            ],
        ],
    )?;
    assert!(
        output.contains("Replacement ADR not found: ADR-9999"),
        "{output}"
    );
    Ok(())
}

#[test]
fn test_accept_rejected_adr_fails() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Bad Decision"],
            &["adr", "reject", "ADR-0001"],
            &["adr", "accept", "ADR-0001"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_accept_nonexistent_adr() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["adr", "accept", "ADR-9999"]])?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
