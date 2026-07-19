use super::*;

// ============================================================================
// RFC Field Edit Tests
// ============================================================================

#[test]
fn test_rfc_set_title() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Original Title"],
            &["rfc", "set", "RFC-0001", "title", "New Title"],
            &["rfc", "list"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_get_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "get", "RFC-0001", "title"],
            &["rfc", "get", "RFC-0001", "status"],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_add_owner() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@newowner"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_remove_owner() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@owner1"],
            &["rfc", "add", "RFC-0001", "owners", "@owner2"],
            &["rfc", "remove", "RFC-0001", "owners", "@owner1"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_remove_owner_by_index_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@owner1"],
            &["rfc", "add", "RFC-0001", "owners", "@owner2"],
            &["rfc", "edit", "RFC-0001", "owners[1]", "--remove"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;

    assert!(
        output.contains("Removed '@owner1' from RFC-0001.owners"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 owners\n@test-user, @owner2"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_add_ref() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "new", "Referenced RFC"],
            &["rfc", "add", "RFC-0001", "refs", "RFC-0002"],
            &["rfc", "get", "RFC-0001", "refs"],
        ],
    )?;
    assert!(
        output.contains("Added 'RFC-0002' to RFC-0001.refs"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 refs\nRFC-0002"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_refs_reject_invalid_hierarchy_and_preserve_existing_value() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "new", "Referenced RFC"],
            &["adr", "new", "Lower Authority Decision"],
            &["rfc", "add", "RFC-0001", "refs", "ADR-0001"],
            &["rfc", "add", "RFC-0001", "refs", "RFC-0002"],
            &["rfc", "edit", "RFC-0001", "refs[0]", "--set", "ADR-0001"],
            &["rfc", "get", "RFC-0001", "refs"],
        ],
    )?;

    assert!(output.contains("error[E0112]"), "output: {}", output);
    assert!(
        !output.contains("Added 'ADR-0001' to RFC-0001.refs"),
        "output: {}",
        output
    );
    assert!(
        !output.contains("Set RFC-0001.refs[0] = ADR-0001"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 refs\nRFC-0002"),
        "output: {}",
        output
    );
    let rfc = std::fs::read_to_string(temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml"))?;
    assert!(!rfc.contains("ADR-0001"), "rfc.toml: {}", rfc);
    Ok(())
}

#[test]
fn test_rfc_edit_set_title_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Original Title"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "title",
                "--set",
                "Canonical Title",
            ],
            &["rfc", "get", "RFC-0001", "title"],
        ],
    )?;

    assert!(
        output.contains("Set RFC-0001.title = Canonical Title"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 title\nCanonical Title"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_edit_set_owner_by_index_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@owner1"],
            &["rfc", "add", "RFC-0001", "owners", "@owner2"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "owners[1]",
                "--set",
                "@replacement",
            ],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    )?;

    assert!(
        output.contains("Set RFC-0001.owners[1] = @replacement"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl rfc get RFC-0001 owners\n@test-user, @replacement"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_set_nonexistent_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "nonexistent", "value"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_set_version_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "version", "0.2.0"],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(
        output.contains("Use `govctl rfc bump`"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_rfc_set_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "status", "normative"],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl rfc finalize"), "output: {}", output);
    Ok(())
}

#[test]
fn test_rfc_get_nonexistent() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "get", "RFC-9999", "title"]])?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_current_changelog_edit_resolves_by_version_and_preserves_lifecycle_fields()
-> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut before: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;
    let current = before["changelog"]
        .as_array()
        .and_then(|entries| entries.first())
        .cloned()
        .ok_or("missing current changelog entry")?;
    let mut historical = current;
    historical["version"] = toml::Value::String("0.0.1".to_string());
    historical["date"] = toml::Value::String("2026-01-01".to_string());
    historical["notes"] = toml::Value::String("Historical summary".to_string());
    before["changelog"]
        .as_array_mut()
        .ok_or("changelog is not an array")?
        .insert(0, historical);
    std::fs::write(&rfc_path, toml::to_string_pretty(&before)?)?;

    let baseline: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.summary",
                "--set",
                "Corrected summary",
            ],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.fixed",
                "--add",
                "First fix",
            ],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.fixed",
                "--add",
                "Second fix",
            ],
            &["rfc", "edit", "RFC-0001", "changelog.fixed[0]", "--remove"],
            &["rfc", "get", "RFC-0001", "changelog"],
        ],
    )?;

    assert!(output.contains("\"summary\": \"Corrected summary\""));
    assert!(output.contains("\"Second fix\""));
    assert!(!output.contains("\"First fix\""));

    let after: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;
    for field in ["version", "phase", "signature"] {
        assert_eq!(after["govctl"][field], baseline["govctl"][field]);
    }
    assert_eq!(after["changelog"][0], baseline["changelog"][0]);
    assert_eq!(
        after["changelog"][1]["date"],
        baseline["changelog"][1]["date"]
    );
    assert_eq!(
        after["changelog"][1]["notes"].as_str(),
        Some("Corrected summary")
    );
    assert_eq!(
        after["changelog"][1]["fixed"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(toml::Value::as_str),
        Some("Second fix")
    );
    Ok(())
}

#[test]
fn test_rfc_changelog_edits_reject_legacy_signature_without_false_amendment() -> common::TestResult
{
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Legacy signature RFC"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.fixed",
                "--add",
                "Existing fix",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "render", "RFC-0001"],
        ],
    )?;

    let rendered = std::fs::read_to_string(temp_dir.path().join("docs/rfc/RFC-0001.md"))?;
    let legacy_signature = rendered
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("<!-- SIGNATURE: sha256:")
                .and_then(|value| value.strip_suffix(" -->"))
        })
        .ok_or("missing rendered RFC signature")?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut rfc: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;
    rfc["govctl"]
        .as_table_mut()
        .ok_or("RFC metadata is not a table")?
        .insert(
            "signature".to_string(),
            toml::Value::String(legacy_signature.to_string()),
        );
    std::fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;
    let before: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.summary",
                "--set",
                "Must not be written",
            ],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.fixed",
                "--add",
                "Must not be added",
            ],
            &["rfc", "edit", "RFC-0001", "changelog.fixed[0]", "--remove"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "False amendment",
            ],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    )?;

    assert_eq!(
        output.matches("error[E0505]").count(),
        3,
        "output: {output}"
    );
    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(output.contains("phase=spec"), "output: {output}");
    assert!(
        output.contains("$ govctl rfc get RFC-0001 phase\nimpl"),
        "output: {output}"
    );
    let after: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;
    assert_eq!(after["changelog"], before["changelog"]);
    Ok(())
}

#[test]
fn test_rfc_changelog_edit_rejects_historical_and_lifecycle_owned_paths() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]])?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let before = std::fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.version",
                "--set",
                "9.9.9",
            ],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.date",
                "--set",
                "2026-01-01",
            ],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog[0].summary",
                "--set",
                "Historical rewrite",
            ],
            &["rfc", "get", "RFC-0001", "changelog.summary"],
            &["rfc", "edit", "RFC-0001", "version", "--set", "9.9.9"],
        ],
    )?;

    assert!(output.contains("Unknown nested field 'version'"));
    assert!(output.contains("Unknown nested field 'date'"));
    assert!(output.contains("Cannot index into non-list field 'changelog'"));
    assert!(output.contains("Path does not support verb 'get'"));
    assert!(output.contains("RFC version is lifecycle-owned"));
    assert_eq!(std::fs::read(&rfc_path)?, before);
    Ok(())
}

#[test]
fn test_rfc_changelog_edit_rejects_duplicate_current_entries_without_mutation() -> common::TestResult
{
    let temp_dir = init_project()?;
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]])?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut malformed: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;
    let duplicate = malformed["changelog"]
        .as_array()
        .and_then(|entries| entries.first())
        .cloned()
        .ok_or("missing current changelog entry")?;
    malformed["changelog"]
        .as_array_mut()
        .ok_or("changelog is not an array")?
        .push(duplicate);
    std::fs::write(&rfc_path, toml::to_string_pretty(&malformed)?)?;
    let before = std::fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "edit",
            "RFC-0001",
            "changelog.fixed",
            "--add",
            "Must not be written",
        ]],
    )?;

    assert!(output.contains("error[E0115]"), "output: {output}");
    assert!(output.contains("found 2"), "output: {output}");
    assert_eq!(std::fs::read(&rfc_path)?, before);
    Ok(())
}

#[test]
fn test_rfc_changelog_operations_reject_missing_current_entry_without_mutation()
-> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]])?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut malformed: toml::Value = toml::from_str(&std::fs::read_to_string(&rfc_path)?)?;
    malformed["govctl"]["version"] = toml::Value::String("9.9.9".to_string());
    std::fs::write(&rfc_path, toml::to_string_pretty(&malformed)?)?;
    let before = std::fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "get", "RFC-0001", "changelog"],
            &[
                "rfc",
                "edit",
                "RFC-0001",
                "changelog.summary",
                "--set",
                "Must not be written",
            ],
        ],
    )?;

    assert_eq!(
        output.matches("error[E0111]").count(),
        2,
        "output: {output}"
    );
    assert!(output.contains("found 0"), "output: {output}");
    assert_eq!(std::fs::read(&rfc_path)?, before);
    Ok(())
}
