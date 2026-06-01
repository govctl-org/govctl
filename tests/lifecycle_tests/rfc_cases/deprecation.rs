use super::*;

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
