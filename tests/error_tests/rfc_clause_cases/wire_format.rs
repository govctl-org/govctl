use super::*;

// =============================================================================
// New wire-format TOML tests ([govctl] layout)
// =============================================================================

fn rfc_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join("gov/rfc/RFC-0001")
}

fn ensure_rfc_dir(root: &std::path::Path) -> common::TestResult {
    fs::create_dir_all(rfc_dir(root).join("clauses"))?;
    Ok(())
}

fn write_rfc_toml(root: &std::path::Path, content: &str) -> common::TestResult {
    ensure_rfc_dir(root)?;
    fs::write(rfc_dir(root).join("rfc.toml"), content)?;
    Ok(())
}

fn write_clause_toml(root: &std::path::Path, file_name: &str, content: &str) -> common::TestResult {
    ensure_rfc_dir(root)?;
    fs::write(rfc_dir(root).join("clauses").join(file_name), content)?;
    Ok(())
}

fn normalized_check_output(
    root: &std::path::Path,
    date: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = run_commands(root, &[&["check"]])?;
    Ok(normalize_output(&output, root, date)?)
}

/// Test: Valid RFC TOML in [govctl] wire format passes check
#[test]
fn test_valid_rfc_toml_wire_format() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Valid clause TOML in [govctl]+[content] wire format passes check
#[test]
fn test_valid_clause_toml_wire_format() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
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

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}

/// Test: RFC TOML in wire format rejects unknown fields in [govctl]
#[test]
fn test_invalid_rfc_toml_wire_unknown_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Clause TOML in wire format rejects unknown fields in [content]
#[test]
fn test_invalid_clause_toml_wire_unknown_field() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    write_clause_toml(
        temp_dir.path(),
        "C-BAD.toml",
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

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}

/// Test: RFC TOML in wire format rejects missing required field (owners)
#[test]
fn test_invalid_rfc_toml_wire_missing_required() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Clause TOML in wire format rejects missing [content].text
#[test]
fn test_invalid_clause_toml_wire_missing_text() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    write_clause_toml(
        temp_dir.path(),
        "C-NOTEXT.toml",
        r#"[govctl]
schema = 1
id = "C-NOTEXT"
title = "No text"
kind = "normative"

[content]
"#,
    )?;

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Legacy flat RFC TOML is still accepted via normalization
#[test]
fn test_legacy_flat_rfc_toml_accepted() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}

/// Test: Legacy flat clause TOML is still accepted via normalization
#[test]
fn test_legacy_flat_clause_toml_accepted() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
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

    write_clause_toml(
        temp_dir.path(),
        "C-FLAT.toml",
        r#"clause_id = "C-FLAT"
title = "Flat Clause"
kind = "normative"
status = "active"
text = "Legacy flat format body."
"#,
    )?;

    assert_error_snapshot!(normalized_check_output(temp_dir.path(), &date)?);
    Ok(())
}
