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
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Test RFC patch amendment",
            ],
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
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Test RFC minor amendment",
            ],
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
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Test RFC major amendment",
            ],
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
fn test_version_bump_rejects_draft_and_preserves_pending_clause() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Draft RFC"],
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
    let rfc_before = fs::read(&rfc_path)?;
    let clause_before = fs::read(&clause_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--patch",
            "--summary",
            "Must be rejected",
        ]],
    )?;

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(
        output.contains("Version-changing bumps require normative RFC status"),
        "output: {output}"
    );
    assert!(!output.contains("Bumped RFC-0001"), "output: {output}");
    assert!(!output.contains("Set C-PENDING.since"), "output: {output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert_eq!(fs::read(&clause_path)?, clause_before);
    let clause: toml::Value = toml::from_str(&fs::read_to_string(&clause_path)?)?;
    assert!(clause["govctl"].get("since").is_none());
    Ok(())
}

#[test]
fn test_version_bump_rejects_deprecated_without_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Deprecated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001", "--force"],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before = fs::read(&rfc_path)?;
    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--patch",
            "--summary",
            "Must be rejected",
        ]],
    )?;

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(output.contains("status=deprecated"), "output: {output}");
    assert!(!output.contains("Bumped RFC-0001"), "output: {output}");
    assert_eq!(fs::read(&rfc_path)?, before);
    Ok(())
}

#[test]
fn test_version_bump_rejects_spec_without_mutation_but_allows_changelog_change()
-> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Spec RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Amended spec candidate",
            ],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before = fs::read(&rfc_path)?;
    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--minor",
            "--summary",
            "Must be rejected",
        ]],
    )?;

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(output.contains("phase=spec"), "output: {output}");
    assert!(!output.contains("Bumped RFC-0001"), "output: {output}");
    assert_eq!(fs::read(&rfc_path)?, before);

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--change",
                "fix: Correct candidate changelog",
            ],
            &["rfc", "get", "RFC-0001", "version"],
        ],
    )?;
    assert!(!output.contains("error["), "output: {output}");
    assert!(
        output.contains("$ govctl rfc get RFC-0001 version\n0.1.0"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_changelog_only_bump_remains_available_for_non_normative_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Draft RFC"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--change",
                "fix: Draft changelog correction",
            ],
            &["rfc", "new", "Deprecated RFC"],
            &["rfc", "finalize", "RFC-0002", "normative"],
            &["rfc", "deprecate", "RFC-0002", "--force"],
            &[
                "rfc",
                "bump",
                "RFC-0002",
                "--change",
                "fix: Deprecated changelog correction",
            ],
        ],
    )?;

    assert!(!output.contains("error["), "output: {output}");
    assert!(
        output.contains("Added change to RFC-0001 v0.1.0"),
        "output: {output}"
    );
    assert!(
        output.contains("Added change to RFC-0002 v0.1.0"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_content_bump_restarts_stable_rfc_at_spec() -> common::TestResult {
    let temp_dir = init_project()?;

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
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "--text",
                "Updated normative behavior.",
            ],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Release amendment",
            ],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;

    assert!(
        output.contains("$ govctl rfc get RFC-0001 phase\nspec"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_bump_preserves_signature_until_new_candidate_advances_to_impl() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let sealed: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    let sealed_signature = sealed["govctl"]["signature"]
        .as_str()
        .ok_or("missing sealed signature")?
        .to_string();

    run_commands(
        temp_dir.path(),
        &[
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Amended Test RFC",
            ],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Release amendment",
            ],
        ],
    )?;

    let candidate: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    assert_eq!(candidate["govctl"]["phase"].as_str(), Some("spec"));
    assert_eq!(
        candidate["govctl"]["signature"].as_str(),
        Some(sealed_signature.as_str())
    );

    run_commands(
        temp_dir.path(),
        &[["rfc", "advance", "RFC-0001", "impl"].as_slice()],
    )?;
    let implemented: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    assert_eq!(implemented["govctl"]["phase"].as_str(), Some("impl"));
    assert_ne!(
        implemented["govctl"]["signature"].as_str(),
        Some(sealed_signature.as_str())
    );
    Ok(())
}

#[test]
fn test_changelog_only_update_preserves_stable_phase() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &["rfc", "get", "RFC-0001", "phase"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--change",
                "fixed: Add changelog note",
            ],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;

    assert_eq!(
        output
            .matches("$ govctl rfc get RFC-0001 phase\nstable")
            .count(),
        2,
        "output: {output}"
    );
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
fn test_changelog_only_bump_rejects_summary_without_level() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]])?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before = fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--summary",
            "Must not be ignored",
            "--change",
            "fix: Must not be recorded",
        ]],
    )?;

    assert!(output.contains("error[E0108]"), "output: {output}");
    assert!(output.contains("Bump level"), "output: {output}");
    assert_eq!(fs::read(&rfc_path)?, before);
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
fn test_bump_change_resolves_reordered_current_entry_without_version_change() -> common::TestResult
{
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    let current = rfc["changelog"]
        .as_array()
        .and_then(|entries| entries.first())
        .cloned()
        .ok_or("missing current changelog entry")?;
    let mut historical = current;
    historical["version"] = toml::Value::String("0.0.1".to_string());
    historical["date"] = toml::Value::String("2026-01-01".to_string());
    historical["notes"] = toml::Value::String("Historical summary".to_string());
    rfc["changelog"]
        .as_array_mut()
        .ok_or("changelog is not an array")?
        .insert(0, historical);
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;
    let before: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--change",
            "fix: Current-only correction",
        ]],
    )?;

    assert!(
        output.contains("Added change to RFC-0001 v0.1.0"),
        "output: {output}"
    );
    let after: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    for field in ["version", "phase", "signature"] {
        assert_eq!(after["govctl"][field], before["govctl"][field]);
    }
    assert_eq!(after["changelog"][0], before["changelog"][0]);
    assert_eq!(
        after["changelog"][1]["date"],
        before["changelog"][1]["date"]
    );
    assert_eq!(
        after["changelog"][1]["fixed"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(toml::Value::as_str),
        Some("Current-only correction")
    );
    Ok(())
}

#[test]
fn test_bump_change_rejects_legacy_signature_without_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Legacy signature RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "render", "RFC-0001"],
        ],
    )?;

    let rendered = fs::read_to_string(temp_dir.path().join("docs/rfc/RFC-0001.md"))?;
    let legacy_signature = rendered
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("<!-- SIGNATURE: sha256:")
                .and_then(|value| value.strip_suffix(" -->"))
        })
        .ok_or("missing rendered RFC signature")?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    rfc["govctl"]
        .as_table_mut()
        .ok_or("RFC metadata is not a table")?
        .insert(
            "signature".to_string(),
            toml::Value::String(legacy_signature.to_string()),
        );
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;
    let before = fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--change",
            "fix: Must not rewrite signature",
        ]],
    )?;

    assert!(output.contains("error[E0505]"), "output: {output}");
    assert!(
        output.contains("legacy amendment signature"),
        "output: {output}"
    );
    assert_eq!(fs::read(&rfc_path)?, before);
    Ok(())
}

#[test]
fn test_version_bump_rejects_missing_signature_and_preserves_pending_clause() -> common::TestResult
{
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    rfc["govctl"]["phase"] = toml::Value::String("stable".to_string());
    rfc["govctl"]
        .as_table_mut()
        .ok_or("RFC metadata is not a table")?
        .remove("signature");
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;
    run_commands(
        temp_dir.path(),
        &[&[
            "clause",
            "new",
            "RFC-0001:C-PENDING",
            "Pending Clause",
            "-s",
            "Specification",
            "-k",
            "normative",
        ]],
    )?;

    let clause_path = temp_dir
        .path()
        .join("gov/rfc/RFC-0001/clauses/C-PENDING.toml");
    let rfc_before = fs::read(&rfc_path)?;
    let clause_before = fs::read(&clause_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--patch",
            "--summary",
            "Release pending clause",
        ]],
    )?;

    assert!(output.contains("error[E0505]"), "output: {output}");
    assert!(
        output.contains("sealed RFC content signature"),
        "output: {output}"
    );
    assert!(output.contains("govctl migrate"), "output: {output}");
    assert!(!output.contains("Bumped RFC-0001"), "output: {output}");
    assert!(!output.contains("Set C-PENDING.since"), "output: {output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert_eq!(fs::read(&clause_path)?, clause_before);
    let clause: toml::Value = toml::from_str(&fs::read_to_string(&clause_path)?)?;
    assert!(clause["govctl"].get("since").is_none());
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
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
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
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
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
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
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
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
        ],
    )?;

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
