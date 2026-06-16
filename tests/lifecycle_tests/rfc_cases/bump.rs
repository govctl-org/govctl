use super::*;

// ============================================================================
// RFC Bump Tests
// ============================================================================

#[test]
fn test_bump_patch_version() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
fn test_bump_rejects_empty_bump_after_signature_baseline() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

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
                "Establish baseline",
            ],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Empty bump",
            ],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_rejects_changelog_only_after_signature_baseline() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

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
                "Establish baseline",
            ],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--change",
                "Added changelog note",
            ],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Changelog-only bump",
            ],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_change_does_not_clear_pending_amendment() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-TEST",
                "Test Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "--text",
                "Original normative behavior.",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Establish baseline",
            ],
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "--text",
                "Updated normative behavior.",
            ],
            &["rfc", "bump", "RFC-0001", "--change", "Added release note"],
            &["rfc", "list"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Release amendment",
            ],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_nonexistent_rfc() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "bump", "RFC-9999", "--patch", "--summary", "Fix"]],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
