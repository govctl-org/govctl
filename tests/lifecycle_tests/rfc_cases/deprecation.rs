use super::*;

// ============================================================================
// RFC Deprecate/Supersede Tests
// ============================================================================

#[test]
fn test_deprecate_normative_rfc() -> common::TestResult {
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
fn test_deprecate_rfc_preserves_pending_clause_version() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &[
                "clause",
                "new",
                "RFC-0001:C-PENDING",
                "Pending Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
        ],
    )?;

    let clause_path = temp_dir
        .path()
        .join("gov/rfc/RFC-0001/clauses/C-PENDING.toml");
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before = fs::read(&clause_path)?;
    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "deprecate", "RFC-0001", "--force"]],
    )?;

    assert!(!output.contains("error["), "output: {output}");
    assert!(output.ends_with("exit: 0\n\n"), "output: {output}");
    assert!(!output.contains("Set C-PENDING.since"), "output: {output}");
    assert_eq!(fs::read(&clause_path)?, before);
    let clause: toml::Value = toml::from_str(&fs::read_to_string(&clause_path)?)?;
    assert!(clause["govctl"].get("since").is_none());
    let rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    assert_eq!(rfc["govctl"]["status"].as_str(), Some("deprecated"));
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
fn test_supersede_rejects_impl_phase_without_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
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

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(
        output.contains("Advance the current version to stable first"),
        "output: {output}"
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 status\nnormative"),
        "output: {output}"
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0002 supersedes\n\nexit: 0"),
        "output: {output}"
    );
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
