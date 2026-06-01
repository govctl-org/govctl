use super::*;

// ============================================================================
// Clause Legacy Format Tests
// ============================================================================

#[test]
fn test_deprecate_legacy_json_clause_requires_migrate() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let clauses_dir = temp_dir.path().join("gov/rfc/RFC-0001/clauses");
    fs::create_dir_all(&clauses_dir)?;
    fs::write(
        clauses_dir.join("C-TEST.json"),
        r#"{
  "clause_id": "C-TEST",
  "title": "Legacy Clause",
  "kind": "normative",
  "status": "active",
  "text": "Legacy clause content."
}"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "deprecate", "RFC-0001:C-TEST", "--force"]],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
// ============================================================================
// Clause Supersede Tests
// ============================================================================

#[test]
fn test_supersede_clause() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-OLD",
                "Old Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "new",
                "RFC-0001:C-NEW",
                "New Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-NEW",
            ],
            &["clause", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_supersede_clause_rejects_missing_replacement() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-OLD",
                "Old Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-MISSING",
                "--force",
            ],
        ],
    )?;
    assert!(output.contains("error[E0202]: Replacement clause not found: RFC-0001:C-MISSING"));
    Ok(())
}

#[test]
fn test_deprecate_clause_force() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Deprecate Clause RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-ONE",
                "Clause One",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "deprecate", "RFC-0001:C-ONE", "--force"],
            &["clause", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_deprecate_clause_already_deprecated_fails() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Deprecate Twice RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-ONE",
                "Clause One",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "deprecate", "RFC-0001:C-ONE", "--force"],
            &["clause", "deprecate", "RFC-0001:C-ONE", "--force"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_deprecate_clause_superseded_fails() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Supersede Then Deprecate RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-OLD",
                "Old Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "new",
                "RFC-0001:C-NEW",
                "New Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-NEW",
                "--force",
            ],
            &["clause", "deprecate", "RFC-0001:C-OLD", "--force"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
