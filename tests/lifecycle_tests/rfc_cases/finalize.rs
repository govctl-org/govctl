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
fn test_finalize_normative_to_deprecated() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
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
