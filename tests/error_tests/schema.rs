/// Test: RFC files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_rfc_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Invalid RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [],
  "unexpected": true
}"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0101]"), "output: {}", output);
    assert!(output.contains("rfc.schema.json"), "output: {}", output);
    Ok(())
}

/// Test: Clause files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_clause_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Clause Schema Test",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [{"title": "Test", "clauses": ["clauses/C-TEST.json"]}]
}"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-TEST.json"),
        r#"{
  "clause_id": "C-TEST",
  "title": "Invalid Clause",
  "kind": "normative",
  "text": "Clause text",
  "unexpected": "should fail schema validation"
}"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0201]"), "output: {}", output);
    assert!(output.contains("clause.schema.json"), "output: {}", output);
    Ok(())
}

#[test]
fn test_invalid_adr_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir.path().join("gov/adr/ADR-0001-invalid.toml"),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Invalid ADR"
status = "accepted"
date = "2026-01-01"

[content]
context = "Context"
decision = "Decision"
consequences = "Consequences"
unexpected = "should fail schema validation"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0301]"), "output: {}", output);
    assert!(output.contains("adr.schema.json"), "output: {}", output);
    Ok(())
}

/// Test: Work item files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_work_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-invalid.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Invalid Work Item"
status = "queue"
created = "2026-01-01"

[content]
description = "Work description"
unexpected = "should fail schema validation"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0401]"), "output: {}", output);
    assert!(output.contains("work.schema.json"), "output: {}", output);
    Ok(())
}

#[test]
fn test_invalid_release_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        r#"[govctl]
schema = 1

[[releases]]
version = "1.0.0"
date = "2026-01-01"
unexpected = "should fail schema validation"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0704]"), "output: {}", output);
    assert!(output.contains("release.schema.json"), "output: {}", output);
    Ok(())
}

/// Test: Verification guard files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_guard_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir.path().join("gov/guard/check.toml"),
        r#"[govctl]
schema = 1
id = "GUARD-CHECK"
title = "Invalid Guard"

[check]
command = "true"
unexpected = "should fail schema validation"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E1001]"), "output: {}", output);
    assert!(output.contains("guard.schema.json"), "output: {}", output);
    Ok(())
}
