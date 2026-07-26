use super::*;

#[test]
fn test_future_project_schema_rejects_mutation_and_migration_without_writes() -> common::TestResult
{
    let temp_dir = init_project()?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let schema_path = temp_dir.path().join("gov/schema/rfc.schema.json");
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(&config_path)?)?;
    let current_version = config["schema"]["version"]
        .as_integer()
        .ok_or("missing schema version")?;
    config["schema"]["version"] = toml::Value::Integer(current_version + 1);
    fs::write(&config_path, toml::to_string_pretty(&config)?)?;
    fs::write(&schema_path, "future schema must remain untouched")?;
    let config_before = fs::read(&config_path)?;
    let schema_before = fs::read(&schema_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "--active", "Must not be created"],
            &["migrate"],
        ],
    )?;

    assert_eq!(output.matches("error[E0505]").count(), 2, "{output}");
    assert!(
        output.contains("newer than this govctl supports"),
        "{output}"
    );
    assert!(output.contains("Upgrade govctl"), "{output}");
    assert_eq!(fs::read(&config_path)?, config_before);
    assert_eq!(fs::read(&schema_path)?, schema_before);
    assert_eq!(fs::read_dir(temp_dir.path().join("gov/work"))?.count(), 0);
    Ok(())
}

#[test]
fn test_missing_project_config_rejects_existing_project_mutation() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "new", "Existing project", "--id", "RFC-0001"]],
    )?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    fs::remove_file(&config_path)?;
    let rfc_before = fs::read(&rfc_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "edit",
            "RFC-0001",
            "title",
            "--set",
            "Must not be written",
        ]],
    )?;

    assert!(output.contains("error[E0505]"), "{output}");
    assert!(output.contains("gov/config.toml is missing"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert!(!config_path.exists());
    Ok(())
}

#[test]
fn test_schema_version_is_validated_before_full_config_deserialization() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "new", "Existing project", "--id", "RFC-0001"]],
    )?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let original_config = fs::read_to_string(&config_path)?;
    let original_rfc = fs::read(&rfc_path)?;

    let mut config: toml::Value = toml::from_str(&original_config)?;
    config["schema"]
        .as_table_mut()
        .ok_or("missing schema table")?
        .remove("version");
    fs::write(&config_path, toml::to_string_pretty(&config)?)?;
    let missing_output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "edit",
            "RFC-0001",
            "title",
            "--set",
            "Must not be written",
        ]],
    )?;
    assert!(missing_output.contains("error[E0501]"), "{missing_output}");
    assert!(
        missing_output.contains("schema.version"),
        "{missing_output}"
    );
    assert_eq!(fs::read(&rfc_path)?, original_rfc);

    let mut config: toml::Value = toml::from_str(&original_config)?;
    let current_version = config["schema"]["version"]
        .as_integer()
        .ok_or("missing schema version")?;
    config["schema"]["version"] = toml::Value::Integer(current_version + 1);
    config
        .as_table_mut()
        .ok_or("config is not a table")?
        .insert(
            "verification".to_string(),
            toml::Value::Table(toml::map::Map::from_iter([(
                "enabled".to_string(),
                toml::Value::String("future".to_string()),
            )])),
        );
    fs::write(&config_path, toml::to_string_pretty(&config)?)?;
    let future_output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "edit",
            "RFC-0001",
            "title",
            "--set",
            "Must not be written",
        ]],
    )?;
    assert!(future_output.contains("error[E0505]"), "{future_output}");
    assert!(future_output.contains("Upgrade govctl"), "{future_output}");
    assert_eq!(fs::read(&rfc_path)?, original_rfc);
    Ok(())
}

#[test]
fn test_explicit_missing_config_does_not_fall_back_to_current_project() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "new", "Existing project", "--id", "RFC-0001"]],
    )?;
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let original_rfc = fs::read(&rfc_path)?;
    let explicit_config = "missing.toml";

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "--config",
            explicit_config,
            "rfc",
            "edit",
            "RFC-0001",
            "title",
            "--set",
            "Must not be written",
        ]],
    )?;

    assert!(output.contains("error[E0502]"), "{output}");
    assert!(output.contains("Configuration file not found"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, original_rfc);
    assert!(!temp_dir.path().join(explicit_config).exists());
    Ok(())
}

#[test]
fn test_missing_project_config_is_found_from_subdirectory() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "new", "Existing project", "--id", "RFC-0001"]],
    )?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    fs::remove_file(&config_path)?;
    let rfc_before = fs::read(&rfc_path)?;
    let nested_dir = temp_dir.path().join("docs/sub");
    fs::create_dir_all(&nested_dir)?;

    let output = run_commands(
        &nested_dir,
        &[&[
            "rfc",
            "edit",
            "RFC-0001",
            "title",
            "--set",
            "Must not be written",
        ]],
    )?;

    assert!(output.contains("error[E0505]"), "{output}");
    assert!(output.contains("gov/config.toml is missing"), "{output}");
    assert_eq!(fs::read(&rfc_path)?, rfc_before);
    assert!(!config_path.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_symlinked_artifact_state_blocks_ancestor_project_selection() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = init_project()?;
    let inner = temp_dir.path().join("inner");
    let inner_rfc_root = inner.join("gov/rfc");
    let external_rfc = temp_dir.path().join("external-rfc");
    fs::create_dir_all(&inner_rfc_root)?;
    fs::create_dir(&external_rfc)?;
    symlink(&external_rfc, inner_rfc_root.join("RFC-0009"))?;
    let outer_work_dir = temp_dir.path().join("gov/work");
    let outer_work_count = fs::read_dir(&outer_work_dir)?.count();

    let output = run_commands(&inner, &[&["work", "new", "Must not use outer project"]])?;

    assert!(output.contains("error[E0505]"), "{output}");
    assert!(output.contains("gov/config.toml is missing"), "{output}");
    assert_eq!(fs::read_dir(&outer_work_dir)?.count(), outer_work_count);
    assert!(!inner.join("gov/config.toml").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_dangling_config_blocks_ancestor_project_selection() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = init_project()?;
    let inner = temp_dir.path().join("inner");
    fs::create_dir_all(inner.join("gov"))?;
    symlink("missing-config.toml", inner.join("gov/config.toml"))?;
    let outer_work_dir = temp_dir.path().join("gov/work");
    let outer_work_count = fs::read_dir(&outer_work_dir)?.count();

    let output = run_commands(&inner, &[&["work", "new", "Must not use outer project"]])?;

    assert!(output.contains("exit: 1"), "{output}");
    assert_eq!(fs::read_dir(&outer_work_dir)?.count(), outer_work_count);
    assert!(!inner.join("gov/work").exists());
    Ok(())
}

#[test]
fn test_schema_state_without_config_blocks_ancestor_project_selection() -> common::TestResult {
    let outer = init_project()?;
    let inner = outer.path().join("inner");
    fs::create_dir(&inner)?;
    run_commands(&inner, &[&["init"]])?;
    fs::remove_file(inner.join("gov/config.toml"))?;
    let outer_work_dir = outer.path().join("gov/work");
    let outer_work_count = fs::read_dir(&outer_work_dir)?.count();

    let output = run_commands(&inner, &[&["work", "new", "Must not use outer project"]])?;

    assert!(output.contains("error[E0505]"), "{output}");
    assert!(output.contains("gov/config.toml is missing"), "{output}");
    assert_eq!(fs::read_dir(&outer_work_dir)?.count(), outer_work_count);
    assert_eq!(fs::read_dir(inner.join("gov/work"))?.count(), 0);
    Ok(())
}

#[test]
fn test_new_resources_reject_empty_titles_before_artifact_write() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(temp_dir.path(), &[&["rfc", "new", "Container RFC"]])?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", ""],
            &["adr", "new", ""],
            &["work", "new", ""],
            &["clause", "new", "RFC-0001:C-EMPTY", ""],
        ],
    )?;

    assert_eq!(output.matches("exit: 1").count(), 4, "{output}");
    assert_eq!(
        output.matches("is shorter than 1 character").count(),
        4,
        "{output}"
    );
    assert!(!temp_dir.path().join("gov/rfc/RFC-0002/rfc.toml").exists());
    assert_eq!(fs::read_dir(temp_dir.path().join("gov/adr"))?.count(), 0);
    assert_eq!(fs::read_dir(temp_dir.path().join("gov/work"))?.count(), 0);
    assert!(
        !temp_dir
            .path()
            .join("gov/rfc/RFC-0001/clauses/C-EMPTY.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn test_generated_resource_ids_reject_exhausted_namespaces() -> common::TestResult {
    let temp_dir = init_project()?;
    fs::write(temp_dir.path().join("gov/adr/ADR-9999-existing.toml"), "")?;
    let date = common::today();
    fs::write(
        temp_dir
            .path()
            .join(format!("gov/work/{date}-existing.toml")),
        format!(
            "#:schema ../schema/work.schema.json\n\n[govctl]\nid    = \"WI-{date}-999\"\ntitle = \"Existing\"\nstatus = \"queue\"\ncreated = \"{date}\"\n\n[content]\ndescription = \"Existing work item\"\n"
        ),
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["adr", "new", "Overflow"], &["work", "new", "Overflow"]],
    )?;

    assert_eq!(output.matches("exit: 1").count(), 2, "{output}");
    assert!(output.contains("ADR-9999"), "{output}");
    assert!(output.contains(&format!("WI-{date}-999")), "{output}");
    assert!(
        !temp_dir
            .path()
            .join("gov/adr/ADR-10000-overflow.toml")
            .exists()
    );
    assert!(
        !temp_dir
            .path()
            .join(format!("gov/work/{date}-overflow.toml"))
            .exists()
    );
    Ok(())
}

#[test]
fn test_legacy_artifact_schema_field_is_rejected_for_all_artifacts() -> common::TestResult {
    let cases = [
        (
            "gov/rfc/RFC-0001/rfc.toml",
            r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Legacy artifact schema field"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["@test-user"]
created = "2026-01-01"

[[sections]]
title = "Specification"
"#,
            "E0101",
        ),
        (
            "gov/rfc/RFC-0002/clauses/C-LEGACY.toml",
            r#"[govctl]
schema = 1
id = "C-LEGACY"
title = "Legacy clause schema field"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "Legacy clause."
"#,
            "E0201",
        ),
        (
            "gov/adr/ADR-0001-legacy.toml",
            r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Legacy ADR schema field"
status = "proposed"
date = "2026-01-01"
refs = []

[content]
context = "Context"
decision = "Decision"
consequences = "Consequences"
"#,
            "E0301",
        ),
        (
            "gov/work/2026-01-01-legacy-schema-field.toml",
            r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Legacy Work Item schema field"
status = "queue"
created = "2026-01-01"

[content]
description = "Description"
"#,
            "E0401",
        ),
        (
            "gov/guard/legacy-schema-field.toml",
            r#"[govctl]
schema = 1
id = "GUARD-LEGACY"
title = "Legacy Guard schema field"

[check]
command = "true"
"#,
            "E1001",
        ),
    ];

    for (relative_path, content, code) in cases {
        let temp_dir = init_project()?;
        let path = temp_dir.path().join(relative_path);
        fs::create_dir_all(path.parent().expect("artifact path has parent"))?;
        if code == "E0201" {
            fs::write(
                temp_dir.path().join("gov/rfc/RFC-0002/rfc.toml"),
                r#"[govctl]
id = "RFC-0002"
title = "Legacy clause schema field"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["@test-user"]
created = "2026-01-01"

[[sections]]
title = "Specification"
clauses = ["clauses/C-LEGACY.toml"]
"#,
            )?;
        }
        fs::write(path, content)?;

        let output = run_commands(temp_dir.path(), &[&["check"]])?;
        assert!(
            output.contains(&format!("error[{code}]")),
            "output: {output}"
        );
        assert!(
            output.contains("Additional properties are not allowed ('schema' was unexpected)"),
            "output: {output}"
        );
    }
    Ok(())
}

/// Test: RFC files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_rfc_schema_check() -> common::TestResult {
    let temp_dir = init_project()?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
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

#[test]
fn test_check_uses_root_gitignore_when_run_from_subdirectory() -> common::TestResult {
    let temp_dir = init_project()?;
    let docs_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&docs_dir)?;
    fs::write(docs_dir.join(".gitignore"), "book/\n")?;

    let output = run_commands(&docs_dir, &[&["check"]])?;
    assert!(!output.contains("warning[W0111]"), "output: {}", output);
    Ok(())
}

#[test]
fn test_check_reports_gitignore_read_error() -> common::TestResult {
    let temp_dir = init_project()?;
    let gitignore_path = temp_dir.path().join(".gitignore");
    fs::remove_file(&gitignore_path)?;
    fs::create_dir(&gitignore_path)?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error"), "output: {}", output);
    assert!(output.contains("read .gitignore"), "output: {}", output);
    assert!(output.contains(".gitignore"), "output: {}", output);
    assert!(output.contains("exit: 1"), "output: {}", output);
    Ok(())
}
