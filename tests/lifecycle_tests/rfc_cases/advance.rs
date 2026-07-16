use super::*;

// ============================================================================
// RFC Advance Tests
// ============================================================================

#[test]
fn test_advance_spec_to_impl() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-9999", "impl"]])?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_seals_content_edits_made_during_spec() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
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
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    let baseline_signature = before["govctl"]["signature"].as_str().map(str::to_string);

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "--text",
                "Amended normative behavior.",
            ],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Amended Test RFC",
            ],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "get", "RFC-0001", "phase"],
            &["rfc", "get", "RFC-0001", "title"],
        ],
    )?;

    assert!(!output.contains("error[E0114]"), "output: {output}");
    assert!(
        output.contains("$ govctl rfc get RFC-0001 phase\nimpl"),
        "output: {output}"
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 title\nAmended Test RFC"),
        "output: {output}"
    );
    let after: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    assert_ne!(
        after["govctl"]["signature"].as_str().map(str::to_string),
        baseline_signature
    );
    Ok(())
}

#[test]
fn test_advance_after_impl_rejects_unversioned_content_amendment() -> common::TestResult {
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
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "--text",
                "Unversioned implementation-phase amendment.",
            ],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;

    assert!(output.contains("error[E0114]"), "output: {output}");
    assert!(output.contains("unversioned amendment"), "output: {output}");
    assert!(
        output.contains("$ govctl rfc get RFC-0001 phase\nimpl"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_advance_to_impl_rejects_pending_clause_versions_without_mutation() -> common::TestResult {
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
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    rfc["govctl"]["status"] = toml::Value::String("normative".to_string());
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;
    let rfc_before = fs::read(&rfc_path)?;
    let clause_before = fs::read(&clause_path)?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-0001", "impl"]])?;

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(output.contains("C-PENDING"), "output: {output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert_eq!(fs::read(&clause_path)?, clause_before);
    Ok(())
}

#[test]
fn test_advance_migrates_legacy_signature_before_phase_changes() -> common::TestResult {
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
    let signature = rendered
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("<!-- SIGNATURE: sha256:")
                .and_then(|value| value.strip_suffix(" -->"))
        })
        .ok_or("missing rendered RFC signature")?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    rfc.get_mut("govctl")
        .and_then(toml::Value::as_table_mut)
        .ok_or("RFC govctl section is not a table")?
        .insert(
            "signature".to_string(),
            toml::Value::String(signature.to_string()),
        );
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;

    assert!(!output.contains("error[E0114]"), "output: {output}");
    assert!(
        output.contains("$ govctl rfc get RFC-0001 phase\ntest"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_advance_rejects_deprecated_rfc_entering_impl() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Deprecated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001", "--force"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(
        output.contains("Only normative RFCs can enter implementation phases"),
        "output: {output}"
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 phase\nspec"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_finalize_sets_updated_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

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
    let (temp_dir, date) = init_project_with_date()?;

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
