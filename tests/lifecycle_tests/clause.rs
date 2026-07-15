use super::*;

// ============================================================================
// Clause Legacy Format Tests
// ============================================================================

#[test]
fn test_deprecate_legacy_json_clause_requires_migrate() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
fn test_supersede_clause_accepts_same_rfc_shorthand_replacement() -> common::TestResult {
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
                "C-NEW",
                "--force",
            ],
            &["clause", "get", "RFC-0001:C-OLD", "superseded_by"],
            &["check"],
        ],
    )?;

    assert!(
        output.contains("Superseded clause: RFC-0001:C-OLD\n  Replaced by: C-NEW\nexit: 0"),
        "output: {output}"
    );
    assert!(
        output.contains("$ govctl clause get RFC-0001:C-OLD superseded_by\nC-NEW\nexit: 0"),
        "output: {output}"
    );
    assert!(
        output.ends_with("exit: 0\n\n"),
        "check should accept the shorthand replacement: {output}"
    );
    Ok(())
}

#[test]
fn test_supersede_clause_chain_remains_valid() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
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
            &[
                "clause",
                "new",
                "RFC-0001:C-TWO",
                "Clause Two",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "new",
                "RFC-0001:C-THREE",
                "Clause Three",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-ONE",
                "--by",
                "RFC-0001:C-TWO",
                "--force",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-TWO",
                "--by",
                "RFC-0001:C-THREE",
                "--force",
            ],
            &["clause", "get", "RFC-0001:C-ONE", "superseded_by"],
            &["clause", "get", "RFC-0001:C-TWO", "superseded_by"],
            &["check"],
        ],
    )?;

    assert!(
        output
            .contains("$ govctl clause get RFC-0001:C-ONE superseded_by\nRFC-0001:C-TWO\nexit: 0"),
        "output: {output}"
    );
    assert!(
        output.contains(
            "$ govctl clause get RFC-0001:C-TWO superseded_by\nRFC-0001:C-THREE\nexit: 0"
        ),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 0\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_clause_history_remains_valid_after_target_is_deprecated() -> common::TestResult {
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
                "new",
                "RFC-0001:C-REPLACEMENT",
                "Replacement Clause",
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
                "RFC-0001:C-REPLACEMENT",
                "--force",
            ],
            &["clause", "deprecate", "RFC-0001:C-REPLACEMENT", "--force"],
            &["clause", "get", "RFC-0001:C-OLD", "superseded_by"],
            &["check"],
        ],
    )?;

    assert!(
        output.contains(
            "$ govctl clause get RFC-0001:C-OLD superseded_by\nRFC-0001:C-REPLACEMENT\nexit: 0"
        ),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 0\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_deprecated_clause() -> common::TestResult {
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
                "new",
                "RFC-0001:C-REPLACEMENT",
                "Replacement Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "deprecate", "RFC-0001:C-OLD", "--force"],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-REPLACEMENT",
                "--force",
            ],
            &["clause", "get", "RFC-0001:C-OLD", "superseded_by"],
            &["check"],
        ],
    )?;

    assert!(
        output.contains(
            "$ govctl clause get RFC-0001:C-OLD superseded_by\nRFC-0001:C-REPLACEMENT\nexit: 0"
        ),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 0\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_clause_accepts_qualified_cross_rfc_replacement() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Source RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-SOURCE",
                "Source Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["rfc", "new", "Target RFC"],
            &[
                "clause",
                "new",
                "RFC-0002:C-TARGET",
                "Target Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-SOURCE",
                "--by",
                "RFC-0002:C-TARGET",
                "--force",
            ],
            &["clause", "get", "RFC-0001:C-SOURCE", "superseded_by"],
            &["check"],
        ],
    )?;

    assert!(
        output.contains(
            "$ govctl clause get RFC-0001:C-SOURCE superseded_by\nRFC-0002:C-TARGET\nexit: 0"
        ),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 0\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_clause_rejects_unqualified_cross_rfc_replacement() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Source RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-SOURCE",
                "Source Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["rfc", "new", "Target RFC"],
            &[
                "clause",
                "new",
                "RFC-0002:C-TARGET",
                "Target Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-SOURCE",
                "--by",
                "C-TARGET",
                "--force",
            ],
        ],
    )?;

    assert!(
        output.contains("Replacement clause not found: RFC-0001:C-TARGET"),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 1\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_clause_rejects_self_replacement() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
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
            &[
                "clause",
                "supersede",
                "RFC-0001:C-ONE",
                "--by",
                "RFC-0001:C-ONE",
                "--force",
            ],
        ],
    )?;

    assert!(
        output.contains("Cannot supersede a clause with itself: RFC-0001:C-ONE"),
        "output: {output}"
    );
    assert!(output.contains("error[E0212]"), "output: {output}");
    assert!(output.ends_with("exit: 1\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_clause_rejects_deprecated_replacement() -> common::TestResult {
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
                "new",
                "RFC-0001:C-REPLACEMENT",
                "Replacement Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "deprecate", "RFC-0001:C-REPLACEMENT", "--force"],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-REPLACEMENT",
                "--force",
            ],
        ],
    )?;

    assert!(
        output.contains("Cannot supersede by a deprecated clause: RFC-0001:C-REPLACEMENT"),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 1\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_clause_rejects_superseded_replacement() -> common::TestResult {
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
                "new",
                "RFC-0001:C-REPLACEMENT",
                "Replacement Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "new",
                "RFC-0001:C-LATEST",
                "Latest Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-REPLACEMENT",
                "--by",
                "RFC-0001:C-LATEST",
                "--force",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-REPLACEMENT",
                "--force",
            ],
        ],
    )?;

    assert!(
        output.contains("Cannot supersede by a superseded clause: RFC-0001:C-REPLACEMENT"),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 1\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_supersede_clause_rejects_repeated_transition() -> common::TestResult {
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
                "new",
                "RFC-0001:C-FIRST",
                "First Replacement",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "new",
                "RFC-0001:C-SECOND",
                "Second Replacement",
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
                "RFC-0001:C-FIRST",
                "--force",
            ],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-SECOND",
                "--force",
            ],
            &["clause", "get", "RFC-0001:C-OLD", "superseded_by"],
        ],
    )?;

    assert!(
        output.contains(
            "Clause is already superseded. Superseded is terminal; there are no valid transitions"
        ),
        "output: {output}"
    );
    assert!(
        output.contains("error[E0209]: Clause is already superseded. Superseded is terminal; there are no valid transitions (RFC-0001:C-OLD)\nexit: 1"),
        "output: {output}"
    );
    assert!(
        output.contains(
            "$ govctl clause get RFC-0001:C-OLD superseded_by\nRFC-0001:C-FIRST\nexit: 0"
        ),
        "output: {output}"
    );
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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
