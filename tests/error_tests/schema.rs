use super::*;

/// Test: RFC files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_rfc_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Invalid RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"
unexpected = true

[[sections]]
title = "Specification"
"#,
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
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Clause Schema Test"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Test"
clauses = ["clauses/C-TEST.toml"]
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-TEST.toml"),
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Invalid Clause"
kind = "normative"

[content]
text = "Clause text"
unexpected = "should fail schema validation"
"#,
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

#[test]
fn test_check_reports_stale_schema_file_even_when_schema_version_is_current() -> common::TestResult
{
    let temp_dir = init_project()?;

    fs::write(
        temp_dir.path().join("gov/schema/work.schema.json"),
        r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["govctl", "content"],
  "properties": {
    "govctl": {
      "type": "object",
      "required": ["id", "title", "status"],
      "properties": {
        "id": { "type": "string" },
        "title": { "type": "string" },
        "status": { "type": "string" },
        "created": { "type": "string" }
      },
      "additionalProperties": false
    },
    "content": {
      "type": "object",
      "properties": {
        "description": { "type": "string" }
      },
      "additionalProperties": true
    }
  },
  "additionalProperties": false
}
"#,
    )?;

    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-dependency.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Dependency field"
status = "queue"
created = "2026-01-01"
depends_on = ["WI-2026-01-01-002"]

[content]
description = "Work description"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("warning[W0110]"), "output: {}", output);
    assert!(output.contains("work.schema.json"), "output: {}", output);
    assert!(output.contains("govctl migrate"), "output: {}", output);
    Ok(())
}

#[test]
fn test_check_reports_missing_local_state_gitignore_entry() -> common::TestResult {
    let temp_dir = init_project()?;
    fs::write(temp_dir.path().join(".gitignore"), ".govctl.lock\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("warning[W0111]"), "output: {}", output);
    assert!(output.contains(".gitignore"), "output: {}", output);
    assert!(output.contains(".govctl/"), "output: {}", output);
    assert!(output.contains("govctl migrate"), "output: {}", output);
    Ok(())
}
