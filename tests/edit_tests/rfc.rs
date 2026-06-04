use super::*;

// ============================================================================
// RFC Field Edit Tests
// ============================================================================

#[test]
fn test_rfc_set_title() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Original Title"],
            &["rfc", "set", "RFC-0001", "title", "New Title"],
            &["rfc", "list"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_get_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "get", "RFC-0001", "title"],
            &["rfc", "get", "RFC-0001", "status"],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_add_owner() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@newowner"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_remove_owner() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@owner1"],
            &["rfc", "add", "RFC-0001", "owners", "@owner2"],
            &["rfc", "remove", "RFC-0001", "owners", "@owner1"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_remove_owner_by_index_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@owner1"],
            &["rfc", "add", "RFC-0001", "owners", "@owner2"],
            &["rfc", "edit", "RFC-0001", "owners[1]", "--remove"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;

    assert!(
        output.contains("Removed '@owner1' from RFC-0001.owners"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 owners\n@test-user, @owner2"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_add_ref() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "new", "Referenced RFC"],
            &["rfc", "add", "RFC-0001", "refs", "RFC-0002"],
            &["rfc", "get", "RFC-0001", "refs"],
        ],
    )?;
    assert!(
        output.contains("Added 'RFC-0002' to RFC-0001.refs"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 refs\nRFC-0002"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_refs_reject_invalid_hierarchy_and_preserve_existing_value() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "new", "Referenced RFC"],
            &["adr", "new", "Lower Authority Decision"],
            &["rfc", "add", "RFC-0001", "refs", "ADR-0001"],
            &["rfc", "add", "RFC-0001", "refs", "RFC-0002"],
            &["rfc", "edit", "RFC-0001", "refs[0]", "--set", "ADR-0001"],
            &["rfc", "get", "RFC-0001", "refs"],
        ],
    )?;

    assert!(output.contains("error[E0112]"), "output: {}", output);
    assert!(
        !output.contains("Added 'ADR-0001' to RFC-0001.refs"),
        "output: {}",
        output
    );
    assert!(
        !output.contains("Set RFC-0001.refs[0] = ADR-0001"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 refs\nRFC-0002"),
        "output: {}",
        output
    );
    let rfc = std::fs::read_to_string(temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml"))?;
    assert!(!rfc.contains("ADR-0001"), "rfc.toml: {}", rfc);
    Ok(())
}

#[test]
fn test_rfc_edit_set_title_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Original Title"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Canonical Title",
            ],
            &["rfc", "get", "RFC-0001", "title"],
        ],
    )?;

    assert!(
        output.contains("Set RFC-0001.title = Canonical Title"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 title\nCanonical Title"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_edit_set_owner_by_index_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@owner1"],
            &["rfc", "add", "RFC-0001", "owners", "@owner2"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "owners[1]",
                "--set",
                "@replacement",
            ],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;

    assert!(
        output.contains("Set RFC-0001.owners[1] = @replacement"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 owners\n@test-user, @replacement"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_set_nonexistent_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "nonexistent", "value"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_set_version_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "version", "0.2.0"],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(
        output.contains("Use `govctl rfc bump`"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_set_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "status", "normative"],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl rfc finalize"), "output: {}", output);
    Ok(())
}

#[test]
fn test_rfc_get_nonexistent() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "get", "RFC-9999", "title"]])?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
