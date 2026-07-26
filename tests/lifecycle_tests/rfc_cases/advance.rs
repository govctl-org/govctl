use super::*;

fn create_rfc_with_clause(project: &std::path::Path, clause_id: &str) -> common::TestResult {
    run_commands(
        project,
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                clause_id,
                "Test Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
        ],
    )?;
    Ok(())
}

fn set_only_clause_reference(rfc_path: &std::path::Path, reference: &str) -> common::TestResult {
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(rfc_path)?)?;
    let clause = rfc["sections"]
        .as_array_mut()
        .ok_or("missing RFC sections")?
        .iter_mut()
        .filter_map(|section| section.get_mut("clauses"))
        .filter_map(toml::Value::as_array_mut)
        .flatten()
        .next()
        .ok_or("missing Clause reference")?;
    *clause = toml::Value::String(reference.to_string());
    fs::write(rfc_path, toml::to_string_pretty(&rfc)?)?;
    Ok(())
}

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
                "text",
                "--set",
                "Original normative behavior.",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    let baseline_signature = before["govctl"]
        .get("signature")
        .and_then(toml::Value::as_str)
        .map(str::to_string);
    assert!(baseline_signature.is_none());

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "text",
                "--set",
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
fn test_advance_after_spec_rejects_missing_signature_without_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Missing signature RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
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
    rfc["govctl"]
        .as_table_mut()
        .ok_or("RFC metadata is not a table")?
        .remove("signature");
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;
    let rfc_before = fs::read(&rfc_path)?;
    let clause_before = fs::read(&clause_path)?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-0001", "test"]])?;

    assert!(output.contains("error[E0505]"), "output: {output}");
    assert!(
        output.contains("sealed RFC content signature"),
        "output: {output}"
    );
    assert!(
        output.contains("Restore the sealed baseline from version-control history"),
        "output: {output}"
    );
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert_eq!(fs::read(&clause_path)?, clause_before);
    let clause: toml::Value = toml::from_str(&fs::read_to_string(&clause_path)?)?;
    assert!(clause["govctl"].get("since").is_none());
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
                "text",
                "--set",
                "Original normative behavior.",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "text",
                "--set",
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
fn test_advance_rejects_unversioned_unlisted_clause() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-LISTED",
                "Listed Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "render", "RFC-0001"],
        ],
    )?;

    let clauses_dir = temp_dir.path().join("gov/rfc/RFC-0001/clauses");
    let listed_path = clauses_dir.join("C-LISTED.toml");
    let orphan_path = clauses_dir.join("C-ORPHAN.toml");
    let mut orphan: toml::Value = toml::from_str(&fs::read_to_string(listed_path)?)?;
    orphan["govctl"]["id"] = toml::Value::String("C-ORPHAN".to_string());
    orphan["govctl"]["title"] = toml::Value::String("Unlisted Clause".to_string());
    fs::write(&orphan_path, toml::to_string_pretty(&orphan)?)?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before = fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["check"], &["rfc", "advance", "RFC-0001", "test"]],
    )?;

    assert!(output.contains("2 clauses"), "{output}");
    assert!(output.contains("error[E0601]"), "{output}");
    assert!(output.contains("error[E0114]"), "{output}");
    assert!(output.contains("unversioned amendment"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, before);
    Ok(())
}

#[test]
fn test_referenced_clause_outside_canonical_directory_affects_signature() -> common::TestResult {
    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-ALT")?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    let canonical_path = rfc_dir.join("clauses/C-ALT.toml");
    let alternate_path = rfc_dir.join("C-ALT.toml");
    fs::rename(&canonical_path, &alternate_path)?;
    let rfc_path = rfc_dir.join("rfc.toml");
    set_only_clause_reference(&rfc_path, "C-ALT.toml")?;

    let finalize_output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;
    assert!(
        finalize_output.contains("Finalized RFC-0001"),
        "{finalize_output}"
    );
    let clause: toml::Value = toml::from_str(&fs::read_to_string(&alternate_path)?)?;
    assert_eq!(clause["govctl"]["since"].as_str(), Some("0.1.0"));

    let impl_output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-0001", "impl"]])?;
    assert!(
        impl_output.contains("Advanced RFC-0001 to phase: impl"),
        "{impl_output}"
    );
    let check_output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(check_output.contains("1 clauses"), "{check_output}");

    let mut clause: toml::Value = toml::from_str(&fs::read_to_string(&alternate_path)?)?;
    clause["content"]["text"] = toml::Value::String("Changed after sealing".to_string());
    fs::write(&alternate_path, toml::to_string_pretty(&clause)?)?;
    let before = fs::read(&rfc_path)?;
    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-0001", "test"]])?;

    assert!(output.contains("error[E0114]"), "{output}");
    assert!(output.contains("unversioned amendment"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_finalize_accepts_relative_clause_path_with_colon() -> common::TestResult {
    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-ALT")?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    let alternate_dir = rfc_dir.join("clauses:v1");
    fs::create_dir(&alternate_dir)?;
    let alternate_path = alternate_dir.join("C-ALT.toml");
    fs::rename(rfc_dir.join("clauses/C-ALT.toml"), &alternate_path)?;
    let rfc_path = rfc_dir.join("rfc.toml");
    set_only_clause_reference(&rfc_path, "clauses:v1/C-ALT.toml")?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert!(output.contains("Finalized RFC-0001"), "{output}");
    let clause: toml::Value = toml::from_str(&fs::read_to_string(&alternate_path)?)?;
    assert_eq!(clause["govctl"]["since"].as_str(), Some("0.1.0"));
    Ok(())
}

#[test]
fn test_finalize_rejects_dangling_clause_reference_without_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-MISSING")?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    let rfc_path = rfc_dir.join("rfc.toml");
    fs::remove_file(rfc_dir.join("clauses/C-MISSING.toml"))?;
    let rfc_before = fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["check"], &["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert_eq!(output.matches("error[E0204]").count(), 2, "{output}");
    assert!(!output.contains("All checks passed"), "{output}");
    assert!(!output.contains("Finalized RFC-0001"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    Ok(())
}

#[test]
fn test_finalize_rejects_absolute_clause_path_without_external_write() -> common::TestResult {
    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-EXT")?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    let canonical_path = rfc_dir.join("clauses/C-EXT.toml");
    let external_path = temp_dir.path().join("external-clause.toml");
    fs::rename(&canonical_path, &external_path)?;
    let rfc_path = rfc_dir.join("rfc.toml");
    set_only_clause_reference(&rfc_path, &external_path.display().to_string())?;
    let rfc_before = fs::read(&rfc_path)?;
    let external_before = fs::read(&external_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert!(output.contains("error[E0204]"), "{output}");
    assert!(output.contains("Invalid clause path"), "{output}");
    assert!(!output.contains("Finalized RFC-0001"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert_eq!(fs::read(&external_path)?, external_before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_finalize_rejects_clause_symlink_outside_rfc() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-LINK")?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    let clause_path = rfc_dir.join("clauses/C-LINK.toml");
    let external_path = temp_dir.path().join("external-clause.toml");
    fs::rename(&clause_path, &external_path)?;
    symlink(&external_path, &clause_path)?;
    let rfc_path = rfc_dir.join("rfc.toml");
    let rfc_before = fs::read(&rfc_path)?;
    let external_before = fs::read(&external_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert!(output.contains("error[E0204]"), "{output}");
    assert!(!output.contains("Finalized RFC-0001"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert_eq!(fs::read(&external_path)?, external_before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_finalize_rejects_clause_directory_symlink_outside_rfc() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-LINK")?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    let clauses_dir = rfc_dir.join("clauses");
    let external_dir = temp_dir.path().join("external-clauses");
    fs::rename(&clauses_dir, &external_dir)?;
    symlink(&external_dir, &clauses_dir)?;
    let external_path = external_dir.join("C-LINK.toml");
    let rfc_path = rfc_dir.join("rfc.toml");
    let rfc_before = fs::read(&rfc_path)?;
    let external_before = fs::read(&external_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert!(output.contains("error[E0204]"), "{output}");
    assert!(!output.contains("Finalized RFC-0001"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert_eq!(fs::read(&external_path)?, external_before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_finalize_rejects_rfc_directory_symlink_outside_storage() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-LINK")?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    let external_dir = temp_dir.path().join("external-rfc");
    fs::rename(&rfc_dir, &external_dir)?;
    symlink(&external_dir, &rfc_dir)?;
    let external_rfc = external_dir.join("rfc.toml");
    let external_clause = external_dir.join("clauses/C-LINK.toml");
    let rfc_before = fs::read(&external_rfc)?;
    let clause_before = fs::read(&external_clause)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert!(output.contains("error[E0204]"), "{output}");
    assert!(!output.contains("Finalized RFC-0001"), "{output}");
    assert_eq!(fs::read(&external_rfc)?, rfc_before);
    assert_eq!(fs::read(&external_clause)?, clause_before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_finalize_rejects_rfc_storage_root_symlink_outside_gov() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = init_project()?;
    create_rfc_with_clause(temp_dir.path(), "RFC-0001:C-LINK")?;

    let rfc_root = temp_dir.path().join("gov/rfc");
    let external_root = temp_dir.path().join("external-rfc-root");
    fs::rename(&rfc_root, &external_root)?;
    symlink(&external_root, &rfc_root)?;
    let external_rfc = external_root.join("RFC-0001/rfc.toml");
    let external_clause = external_root.join("RFC-0001/clauses/C-LINK.toml");
    let rfc_before = fs::read(&external_rfc)?;
    let clause_before = fs::read(&external_clause)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;

    assert!(output.contains("error[E0204]"), "{output}");
    assert!(!output.contains("Finalized RFC-0001"), "{output}");
    assert_eq!(fs::read(&external_rfc)?, rfc_before);
    assert_eq!(fs::read(&external_clause)?, clause_before);
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
fn test_advance_to_impl_replaces_existing_signature() -> common::TestResult {
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
fn test_later_phase_advance_treats_mismatched_signature_as_amendment() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Legacy signature RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "render", "RFC-0001"],
        ],
    )?;

    let rendered = fs::read_to_string(temp_dir.path().join("docs/rfc/RFC-0001.md"))?;
    let mismatched_signature = rendered
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("<!-- SIGNATURE: sha256:")
                .and_then(|value| value.strip_suffix(" -->"))
        })
        .ok_or("missing rendered RFC signature")?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    rfc["govctl"]["signature"] = toml::Value::String(mismatched_signature.to_string());
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;
    let before = fs::read(&rfc_path)?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-0001", "test"]])?;

    assert!(output.contains("error[E0114]"), "output: {output}");
    assert!(output.contains("unversioned amendment"), "output: {output}");
    assert_eq!(fs::read(&rfc_path)?, before);
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
