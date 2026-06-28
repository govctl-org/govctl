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

#[test]
fn test_bump_rejects_malformed_unlisted_clause_without_mutation() -> common::TestResult {
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
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--patch",
            "--summary",
            "Rejected bump",
        ]],
    )?;

    assert!(output.contains("error[E0201]"), "output: {output}");
    assert!(!output.contains("Bumped RFC-0001"), "output: {output}");
    assert_eq!(fs::read_to_string(rfc_path)?, original_rfc);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_failed_bump_preserves_files_without_success_output() -> common::TestResult {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
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

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let clause_path = temp_dir
        .path()
        .join("gov/rfc/RFC-0001/clauses/C-PENDING.toml");
    let original_rfc = fs::read_to_string(&rfc_path)?;
    let original_clause = fs::read_to_string(&clause_path)?;
    let clauses_dir = clause_path
        .parent()
        .ok_or("pending clause path has no parent directory")?;
    let original_dir_permissions = fs::metadata(clauses_dir)?.permissions();
    let mut unwritable_dir_permissions = original_dir_permissions.clone();
    unwritable_dir_permissions.set_mode(original_dir_permissions.mode() & !0o222);
    fs::set_permissions(clauses_dir, unwritable_dir_permissions)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--patch",
            "--summary",
            "Rejected bump",
        ]],
    );
    fs::set_permissions(clauses_dir, original_dir_permissions)?;
    let output = output?;

    assert!(output.contains("error[E0901]"), "output: {output}");
    assert!(!output.contains("Bumped RFC-0001"), "output: {output}");
    assert!(!output.contains("Added change:"), "output: {output}");
    assert!(
        !output.contains("Set C-PENDING.since ="),
        "output: {output}"
    );
    assert_eq!(fs::read_to_string(rfc_path)?, original_rfc);
    assert_eq!(fs::read_to_string(clause_path)?, original_clause);
    Ok(())
}
