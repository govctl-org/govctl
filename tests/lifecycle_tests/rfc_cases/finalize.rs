use super::*;

// ============================================================================
// RFC Finalize Tests
// ============================================================================

#[test]
fn test_finalize_draft_to_normative() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "list"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_draft_to_deprecated() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Obsolete RFC"],
            &["rfc", "finalize", "RFC-0001", "deprecated"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_rejects_deprecated_target_for_normative_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Normative RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "finalize", "RFC-0001", "deprecated"],
            &["rfc", "get", "RFC-0001", "status"],
        ],
    )?;

    assert!(output.contains("invalid value 'deprecated'"), "{output}");
    assert!(output.contains("[possible values: normative]"), "{output}");
    assert!(
        output.contains("$ govctl rfc get RFC-0001 status\nnormative"),
        "{output}"
    );
    Ok(())
}

#[test]
fn test_finalize_normative_to_deprecated() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001", "--force"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_deprecate_rejects_impl_phase_without_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "In-progress RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "deprecate", "RFC-0001", "--force"],
            &["rfc", "get", "RFC-0001", "status"],
        ],
    )?;

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(
        output.contains("Advance the current version to stable first"),
        "output: {output}"
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 status\nnormative"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_finalize_already_normative_fails() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_nonexistent_rfc() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-9999", "normative"]],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_rejects_malformed_unlisted_clause_without_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]])?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let original_rfc = fs::read_to_string(&rfc_path)?;
    fs::write(
        temp_dir
            .path()
            .join("gov/rfc/RFC-0001/clauses/C-BROKEN.toml"),
        "not valid TOML [",
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert!(output.contains("error[E0201]"), "output: {output}");
    assert!(!output.contains("Finalized RFC-0001"), "output: {output}");
    assert_eq!(fs::read_to_string(rfc_path)?, original_rfc);
    Ok(())
}

#[test]
fn test_finalize_legacy_json_rfc_requires_migrate() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(&rfc_dir)?;
    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Legacy RFC",
  "version": "0.1.0",
  "status": "draft",
  "phase": "spec",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [],
  "changelog": [{ "version": "0.1.0", "date": "2026-01-01", "notes": "Initial draft" }]
}"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
