//! Tests for versioned storage migration pipeline.

mod common;

use common::{TestResult, init_project, init_project_v1, run_commands};
use std::fs;
use std::io;
use std::path::Path;

fn current_schema_version(dir: &Path) -> Result<u32, Box<dyn std::error::Error>> {
    let config = fs::read_to_string(dir.join("gov/config.toml"))?;
    let parsed: toml::Value = toml::from_str(&config)?;
    let version = parsed
        .get("schema")
        .and_then(|schema| schema.get("version"))
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "missing gov/config.toml schema.version",
            )
        })?;
    Ok(u32::try_from(version)?)
}

fn write_legacy_rfc_project(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let rfc_dir = dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Legacy RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["@test-user"],
  "created": "2026-01-01",
  "refs": [],
  "sections": [
    {
      "title": "Specification",
      "clauses": ["clauses/C-LEGACY.json"]
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

    fs::write(
        rfc_dir.join("clauses/C-LEGACY.json"),
        r#"{
  "clause_id": "C-LEGACY",
  "title": "Legacy Clause",
  "kind": "normative",
  "status": "active",
  "text": "Legacy clause text.",
  "since": "1.0.0"
}"#,
    )?;

    Ok(())
}

#[test]
fn test_migrate_rejects_legacy_json_storage() -> TestResult {
    let temp_dir = init_project_v1()?;
    write_legacy_rfc_project(temp_dir.path())?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(output.contains("error[E0505]"), "output: {}", output);
    assert!(
        output.contains("Migrate this repository with a compatible earlier govctl version"),
        "output: {}",
        output
    );

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    assert!(rfc_dir.join("rfc.json").exists());
    assert!(rfc_dir.join("clauses/C-LEGACY.json").exists());
    assert!(!rfc_dir.join("rfc.toml").exists());
    assert!(!rfc_dir.join("clauses/C-LEGACY.toml").exists());

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 1"),
        "schema version should not be bumped on legacy JSON rejection: {}",
        config
    );

    Ok(())
}

#[test]
fn test_migrate_dry_run_rejects_legacy_json_storage() -> TestResult {
    let temp_dir = init_project_v1()?;
    write_legacy_rfc_project(temp_dir.path())?;

    let output = run_commands(temp_dir.path(), &[&["--dry-run", "migrate"]])?;
    assert!(output.contains("error[E0505]"), "output: {}", output);
    assert!(
        output.contains("Migrate this repository with a compatible earlier govctl version"),
        "output: {}",
        output
    );
    assert!(
        !output.contains("Would write: gov/rfc/RFC-0001/rfc.toml"),
        "output: {}",
        output
    );
    assert!(
        !output.contains("Would delete: gov/rfc/RFC-0001/rfc.json"),
        "output: {}",
        output
    );

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    assert!(rfc_dir.join("rfc.json").exists());
    assert!(rfc_dir.join("clauses/C-LEGACY.json").exists());
    assert!(!rfc_dir.join("rfc.toml").exists());
    assert!(!rfc_dir.join("clauses/C-LEGACY.toml").exists());

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 1"),
        "dry-run should not bump version: {}",
        config
    );

    Ok(())
}

#[test]
fn test_migrate_dry_run_rejects_unsupported_schema_without_artifact_changes() -> TestResult {
    let temp_dir = init_project_v1()?;

    let output = run_commands(temp_dir.path(), &[&["--dry-run", "migrate"]])?;
    assert!(
        output.contains("Project schema version 1 is unsupported (minimum: 3)"),
        "output: {output}"
    );

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 1"),
        "dry-run should not bump version: {config}"
    );

    Ok(())
}

#[test]
fn test_migrate_is_noop_on_current_version() -> TestResult {
    let temp_dir = init_project()?;
    let expected_version = current_schema_version(temp_dir.path())?;

    // Create an RFC so the project isn't empty, then migrate to bump version
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Migrated RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-SUMMARY",
                "Summary",
                "-s",
                "Summary",
            ],
            &["migrate"],
        ],
    )?;

    // Second migrate should be a noop
    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(
        output.contains(&format!("already at schema version {expected_version}")),
        "output: {}",
        output
    );

    Ok(())
}

#[test]
fn test_migrate_syncs_stale_schema_file_at_current_version() -> TestResult {
    let temp_dir = init_project()?;
    let expected_version = current_schema_version(temp_dir.path())?;
    let schema_path = temp_dir.path().join("gov/schema/work.schema.json");
    fs::write(&schema_path, "{}\n")?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(
        output.contains(&format!(
            "Synced 1 schema file(s); already at schema version {expected_version}"
        )),
        "output: {}",
        output
    );
    assert_eq!(
        fs::read_to_string(schema_path)?,
        include_str!("../gov/schema/work.schema.json")
    );

    Ok(())
}

#[test]
fn test_migrate_schema_sync_failure_leaves_earlier_schema_unchanged() -> TestResult {
    let temp_dir = init_project()?;
    let rfc_schema = temp_dir.path().join("gov/schema/rfc.schema.json");
    let clause_schema = temp_dir.path().join("gov/schema/clause.schema.json");
    fs::write(&rfc_schema, "stale schema\n")?;
    fs::remove_file(&clause_schema)?;
    fs::create_dir(&clause_schema)?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;

    assert!(output.contains("exit: 1"), "{output}");
    assert_eq!(fs::read_to_string(&rfc_schema)?, "stale schema\n");
    assert!(clause_schema.is_dir());
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_migrate_noop_does_not_require_schema_write_access() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = init_project()?;
    let schema_path = temp_dir.path().join("gov/schema/rfc.schema.json");
    let original_permissions = fs::metadata(&schema_path)?.permissions();
    fs::set_permissions(&schema_path, fs::Permissions::from_mode(0o444))?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;

    fs::set_permissions(&schema_path, original_permissions)?;
    assert!(output.contains("exit: 0"), "{output}");
    assert!(output.contains("already at schema version"), "{output}");
    Ok(())
}

#[test]
fn test_migrate_dry_run_reports_would_sync_at_current_version() -> TestResult {
    let temp_dir = init_project()?;
    let expected_version = current_schema_version(temp_dir.path())?;
    let schema_path = temp_dir.path().join("gov/schema/work.schema.json");
    fs::write(&schema_path, "{}\n")?;

    let output = run_commands(temp_dir.path(), &[&["--dry-run", "migrate"]])?;
    assert!(
        output.contains(&format!(
            "Would sync 1 schema file(s); already at schema version {expected_version}"
        )),
        "output: {}",
        output
    );
    assert_eq!(fs::read_to_string(schema_path)?, "{}\n");

    Ok(())
}

#[test]
fn test_migrate_syncs_missing_local_state_gitignore_entry_at_current_version() -> TestResult {
    let temp_dir = init_project()?;
    let expected_version = current_schema_version(temp_dir.path())?;
    let gitignore_path = temp_dir.path().join(".gitignore");
    fs::write(&gitignore_path, ".govctl.lock\n")?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(
        output.contains(&format!(
            "Synced 1 gitignore entry; already at schema version {expected_version}"
        )),
        "output: {}",
        output
    );

    let gitignore = fs::read_to_string(gitignore_path)?;
    assert_eq!(
        gitignore.matches(".govctl.lock").count(),
        1,
        "migrate should not duplicate existing lock ignore entry"
    );
    assert!(
        gitignore.lines().any(|line| line.trim() == ".govctl/"),
        "migrate should add .govctl/ to .gitignore: {}",
        gitignore
    );

    Ok(())
}

#[test]
fn test_migrate_rejects_unsupported_schema_even_without_file_changes() -> TestResult {
    let temp_dir = init_project_v1()?;

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(config.contains("version = 1"));

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(
        output.contains("Project schema version 1 is unsupported (minimum: 3)"),
        "output: {output}"
    );

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 1"),
        "unsupported config must remain unchanged: {config}"
    );

    Ok(())
}

#[test]
fn test_migrate_rejects_schema_two_without_rebaselining_signatures() -> TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Legacy signature RFC"],
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
                "Stable normative behavior.",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
        ],
    )?;

    let rfc_path = temp_dir.path().join("gov/rfc/RFC-0001/rfc.toml");
    let mut rfc: toml::Value = toml::from_str(&fs::read_to_string(&rfc_path)?)?;
    rfc.get_mut("govctl")
        .and_then(toml::Value::as_table_mut)
        .ok_or("RFC govctl section is not a table")?
        .insert("signature".to_string(), toml::Value::String("0".repeat(64)));
    fs::write(&rfc_path, toml::to_string_pretty(&rfc)?)?;

    let config_path = temp_dir.path().join("gov/config.toml");
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(&config_path)?)?;
    config["schema"]["version"] = toml::Value::Integer(2);
    fs::write(&config_path, toml::to_string_pretty(&config)?)?;

    let before = fs::read(&rfc_path)?;
    let migrated = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(
        migrated.contains("Project schema version 2 is unsupported (minimum: 3)"),
        "{migrated}"
    );
    assert_eq!(fs::read(&rfc_path)?, before);
    assert_eq!(current_schema_version(temp_dir.path())?, 2);

    Ok(())
}

#[test]
fn test_check_rejects_legacy_json_storage() -> TestResult {
    let temp_dir = init_project_v1()?;
    write_legacy_rfc_project(temp_dir.path())?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0505]"), "output: {}", output);
    assert!(
        output.contains("Migrate this repository with a compatible earlier govctl version"),
        "output: {}",
        output
    );

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    assert!(rfc_dir.join("rfc.json").exists());
    assert!(!rfc_dir.join("rfc.toml").exists());

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 1"),
        "version should not be bumped on failure: {}",
        config
    );

    Ok(())
}
