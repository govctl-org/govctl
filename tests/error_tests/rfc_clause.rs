#[test]
fn test_clause_commands_reject_invalid_clause_id_format() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "clause",
                "new",
                "RFC-0001:C-ONE:EXTRA",
                "Bad Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "show", "RFC-0001:C-ONE:EXTRA"],
            &["clause", "delete", "RFC-0001:C-ONE:EXTRA", "-f"],
            &["clause", "deprecate", "RFC-0001:C-ONE:EXTRA", "-f"],
        ],
    )?;

    assert_eq!(
        output.matches("exit: 1").count(),
        4,
        "all malformed clause commands should fail: {output}"
    );
    assert!(
        output.contains("Invalid clause ID format. Expected RFC-NNNN:C-NAME"),
        "new/delete invalid-ID diagnostics should stay stable: {output}"
    );
    assert!(
        output.contains(
            "Invalid clause ID format: RFC-0001:C-ONE:EXTRA (expected RFC-NNNN:C-NAME)"
        ),
        "show invalid-ID diagnostic should stay stable: {output}"
    );
    assert!(
        output.contains("Clause not found: RFC-0001:C-ONE:EXTRA"),
        "lifecycle malformed-ID lookup should stay in the not-found path: {output}"
    );
    Ok(())
}

#[test]
fn test_broken_superseded_check() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    // Create RFC with broken supersession
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Broken Superseded Test",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Clauses",
      "clauses": ["clauses/C-OLD.json", "clauses/C-NEW.json"]
    }
  ],
  "changelog": [
    {
      "version": "1.0.0",
      "date": "2026-01-01",
      "added": ["Initial release"]
    }
  ]
}"#,
    )?;

    // C-OLD claims to be superseded by C-NONEXISTENT (which doesn't exist)
    fs::write(
        rfc_dir.join("clauses/C-OLD.json"),
        r#"{
  "clause_id": "C-OLD",
  "title": "Old Clause",
  "kind": "normative",
  "status": "superseded",
  "text": "This clause is superseded.",
  "superseded_by": "C-NONEXISTENT",
  "since": "1.0.0"
}"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-NEW.json"),
        r#"{
  "clause_id": "C-NEW",
  "title": "New Clause",
  "kind": "normative",
  "status": "active",
  "text": "This is the new clause.",
  "since": "1.0.0"
}"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: RFC has invalid status/phase combination (draft + stable)
#[test]
fn test_invalid_transition_check() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    // Create RFC with invalid state
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Invalid Transition Test",
  "version": "0.1.0",
  "status": "draft",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Overview",
      "clauses": ["clauses/C-TEST.json"]
    }
  ],
  "changelog": [
    {
      "version": "0.1.0",
      "date": "2026-01-01",
      "added": ["Initial draft"]
    }
  ]
}"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-TEST.json"),
        r#"{
  "clause_id": "C-TEST",
  "title": "Test Clause",
  "kind": "normative",
  "status": "active",
  "text": "A test clause in an invalid RFC.",
  "since": "0.1.0"
}"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// =============================================================================
// New wire-format TOML tests ([govctl] layout)
// =============================================================================

/// Test: Valid RFC TOML in [govctl] wire format passes check
#[test]
fn test_valid_rfc_toml_wire_format() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "Wire Format Test"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Summary"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Valid clause TOML in [govctl]+[content] wire format passes check
#[test]
fn test_valid_clause_toml_wire_format() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "Clause Wire Test"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Spec"
clauses = ["clauses/C-TEST.toml"]
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-TEST.toml"),
        r#"#:schema ../../../schema/clause.schema.json

[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "0.1.0"

[content]
text = "Clause body text."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: RFC TOML in wire format rejects unknown fields in [govctl]
#[test]
fn test_invalid_rfc_toml_wire_unknown_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Bad RFC"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"
unexpected = "extra field"

[[sections]]
title = "Summary"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Clause TOML in wire format rejects unknown fields in [content]
#[test]
fn test_invalid_clause_toml_wire_unknown_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Clause Error Test"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Spec"
clauses = ["clauses/C-BAD.toml"]
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-BAD.toml"),
        r#"[govctl]
schema = 1
id = "C-BAD"
title = "Bad Clause"
kind = "normative"

[content]
text = "Body."
unexpected = "extra field"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: RFC TOML in wire format rejects missing required field (owners)
#[test]
fn test_invalid_rfc_toml_wire_missing_required() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Missing owners"
version = "0.1.0"
status = "draft"
phase = "spec"
created = "2026-01-01"

[[sections]]
title = "Summary"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Clause TOML in wire format rejects missing [content].text
#[test]
fn test_invalid_clause_toml_wire_missing_text() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Missing text test"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Spec"
clauses = ["clauses/C-NOTEXT.toml"]
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-NOTEXT.toml"),
        r#"[govctl]
schema = 1
id = "C-NOTEXT"
title = "No text"
kind = "normative"

[content]
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Legacy flat RFC TOML is still accepted via normalization
#[test]
fn test_legacy_flat_rfc_toml_accepted() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"rfc_id = "RFC-0001"
title = "Flat Format"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Summary"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Legacy flat clause TOML is still accepted via normalization
#[test]
fn test_legacy_flat_clause_toml_accepted() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"rfc_id = "RFC-0001"
title = "Flat Clause Test"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Spec"
clauses = ["clauses/C-FLAT.toml"]
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-FLAT.toml"),
        r#"clause_id = "C-FLAT"
title = "Flat Clause"
kind = "normative"
status = "active"
text = "Legacy flat format body."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
