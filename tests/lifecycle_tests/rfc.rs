// ============================================================================
// RFC Finalize Tests
// ============================================================================

#[test]
fn test_finalize_draft_to_normative() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

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
    let temp_dir = init_project()?;
    let date = today();

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
    let temp_dir = init_project()?;
    let date = today();

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
    let temp_dir = init_project()?;
    let date = today();

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
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-9999", "normative"]],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_legacy_json_rfc_requires_migrate() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

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

// ============================================================================
// RFC Advance Tests
// ============================================================================

#[test]
fn test_advance_spec_to_impl() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_impl_to_test() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_test_to_stable() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_draft_to_impl_fails() -> common::TestResult {
    // Cannot advance draft RFC to impl phase
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "advance", "RFC-0001", "impl"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_skip_phase_fails() -> common::TestResult {
    // Cannot skip phases (e.g., spec -> test)
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "test"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_backwards_fails() -> common::TestResult {
    // Cannot go backwards (e.g., impl -> spec)
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "spec"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-9999", "impl"]])?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_sets_updated_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Updated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "get", "RFC-0001", "updated"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_sets_updated_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Updated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "get", "RFC-0001", "updated"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// RFC Bump Tests
// ============================================================================

#[test]
fn test_bump_patch_version() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Minor fix",
            ],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_minor_version() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--minor",
                "--summary",
                "New feature",
            ],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_major_version() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--major",
                "--summary",
                "Breaking change",
            ],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_requires_summary() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "bump", "RFC-0001", "--patch"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_with_change() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "bump", "RFC-0001", "--change", "Added new clause"],
            &["rfc", "show", "RFC-0001"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "bump", "RFC-9999", "--patch", "--summary", "Fix"]],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// RFC Deprecate/Supersede Tests
// ============================================================================

#[test]
fn test_deprecate_normative_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

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
fn test_supersede_rfc() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "new", "New RFC"],
            &["rfc", "finalize", "RFC-0002", "normative"],
            &[
                "rfc",
                "supersede",
                "RFC-0001",
                "--by",
                "RFC-0002",
                "--force",
            ],
            &["rfc", "get", "RFC-0001", "status"],
            &["rfc", "get", "RFC-0002", "supersedes"],
        ],
    )?;
    assert!(
        output.contains("Superseded RFC: RFC-0001"),
        "output: {output}"
    );
    assert!(output.contains("Replaced by: RFC-0002"), "output: {output}");
    assert!(output.contains("$ govctl rfc get RFC-0001 status\ndeprecated"));
    assert!(output.contains("$ govctl rfc get RFC-0002 supersedes\nRFC-0001"));
    Ok(())
}

#[test]
fn test_supersede_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "New RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "supersede",
                "RFC-9999",
                "--by",
                "RFC-0001",
                "--force",
            ],
        ],
    )?;
    assert!(output.contains("error[E0102]: RFC not found: RFC-9999"));
    Ok(())
}

#[test]
fn test_supersede_rfc_rejects_missing_replacement() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "supersede",
                "RFC-0001",
                "--by",
                "RFC-9999",
                "--force",
            ],
        ],
    )?;
    assert!(output.contains("error[E0102]: Replacement RFC not found: RFC-9999"));
    Ok(())
}

