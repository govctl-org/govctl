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

fn write_schema_four_source_scan(
    dir: &Path,
    exclude: toml::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(&config_path)?)?;
    config["schema"]["version"] = toml::Value::Integer(4);
    config
        .as_table_mut()
        .ok_or("config is not a table")?
        .insert(
            "source_scan".to_string(),
            toml::Value::Table(toml::map::Map::from_iter([
                ("enabled".to_string(), toml::Value::Boolean(true)),
                (
                    "include".to_string(),
                    toml::Value::Array(vec![toml::Value::String("src/**/*.rs".to_string())]),
                ),
                ("exclude".to_string(), exclude),
            ])),
        );
    fs::write(config_path, toml::to_string_pretty(&config)?)?;
    Ok(())
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

#[test]
fn test_migrate_v4_source_excludes_to_govignore() -> TestResult {
    let temp_dir = init_project()?;
    write_schema_four_source_scan(
        temp_dir.path(),
        toml::Value::Array(vec![
            toml::Value::String("target/".to_string()),
            toml::Value::String("!literal".to_string()),
            toml::Value::String("#literal".to_string()),
        ]),
    )?;
    fs::write(temp_dir.path().join(".govignore"), "existing-rule\n")?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(output.contains("v4 -> v5"), "{output}");
    assert_eq!(current_schema_version(temp_dir.path())?, 5);
    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(!config.contains("exclude"), "{config}");
    assert_eq!(
        fs::read_to_string(temp_dir.path().join(".govignore"))?,
        "target/\n\\!literal\n\\#literal\nexisting-rule\n"
    );
    Ok(())
}

#[test]
fn test_migrate_v4_preserves_config_comments_and_section_order() -> TestResult {
    let temp_dir = init_project()?;
    write_schema_four_source_scan(
        temp_dir.path(),
        toml::Value::Array(vec![toml::Value::String("target/".to_string())]),
    )?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let config = fs::read_to_string(&config_path)?
        .replacen("version = 4", "version = 4 # schema version", 1)
        .replacen("[source_scan]", "# source scan settings\n[source_scan]", 1);
    let sections_before = config
        .lines()
        .filter(|line| line.starts_with('['))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    fs::write(&config_path, config)?;

    run_commands(temp_dir.path(), &[&["migrate"]])?;

    let migrated = fs::read_to_string(&config_path)?;
    let sections_after = migrated
        .lines()
        .filter(|line| line.starts_with('['))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(
        migrated.contains("version = 5 # schema version"),
        "{migrated}"
    );
    assert!(migrated.contains("# source scan settings"), "{migrated}");
    assert!(!migrated.contains("exclude"), "{migrated}");
    assert_eq!(sections_after, sections_before);
    Ok(())
}

#[test]
fn test_migrate_v4_empty_excludes_does_not_create_govignore() -> TestResult {
    let temp_dir = init_project()?;
    write_schema_four_source_scan(temp_dir.path(), toml::Value::Array(vec![]))?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(output.contains("v4 -> v5"), "{output}");
    assert_eq!(current_schema_version(temp_dir.path())?, 5);
    assert!(!temp_dir.path().join(".govignore").exists());
    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(!config.contains("exclude"), "{config}");
    Ok(())
}

#[test]
fn test_migrate_v4_source_excludes_dry_run_reports_without_writing() -> TestResult {
    let temp_dir = init_project()?;
    write_schema_four_source_scan(
        temp_dir.path(),
        toml::Value::Array(vec![toml::Value::String("target/".to_string())]),
    )?;
    fs::write(temp_dir.path().join(".govignore"), "existing-rule\n")?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let govignore_path = temp_dir.path().join(".govignore");
    let config_before = fs::read(&config_path)?;
    let govignore_before = fs::read(&govignore_path)?;
    let lock_path = temp_dir.path().join("gov/.govctl.lock");
    if lock_path.exists() {
        fs::remove_file(&lock_path)?;
    }

    let output = run_commands(temp_dir.path(), &[&["--dry-run", "migrate"]])?;
    assert!(output.contains("Would write: .govignore"), "{output}");
    assert!(output.contains("Would write: gov/config.toml"), "{output}");
    assert_eq!(fs::read(&config_path)?, config_before);
    assert_eq!(fs::read(&govignore_path)?, govignore_before);
    assert!(!lock_path.exists());
    Ok(())
}

#[test]
fn test_migrate_v4_rejects_multiline_exclude_without_mutation() -> TestResult {
    let temp_dir = init_project()?;
    write_schema_four_source_scan(
        temp_dir.path(),
        toml::Value::Array(vec![toml::Value::String("bad\nrule".to_string())]),
    )?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let config_before = fs::read(&config_path)?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(output.contains("source_scan.exclude[0]"), "{output}");
    assert_eq!(fs::read(&config_path)?, config_before);
    assert!(!temp_dir.path().join(".govignore").exists());
    Ok(())
}

#[test]
fn test_schema_v5_rejects_residual_source_scan_exclude() -> TestResult {
    let temp_dir = init_project()?;
    let config_path = temp_dir.path().join("gov/config.toml");
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(&config_path)?)?;
    config
        .as_table_mut()
        .ok_or("config is not a table")?
        .insert(
            "source_scan".to_string(),
            toml::Value::Table(toml::map::Map::from_iter([(
                "exclude".to_string(),
                toml::Value::Array(vec![]),
            )])),
        );
    fs::write(&config_path, toml::to_string_pretty(&config)?)?;
    let before = fs::read(&config_path)?;

    let output = run_commands(temp_dir.path(), &[&["check"], &["migrate"]])?;
    assert_eq!(output.matches("error[E0501]").count(), 2, "{output}");
    assert!(output.contains("source_scan.exclude"), "{output}");
    assert_eq!(fs::read(&config_path)?, before);
    Ok(())
}

#[test]
fn test_schema_v4_normal_command_requires_migration_without_mutation() -> TestResult {
    let temp_dir = init_project()?;
    write_schema_four_source_scan(temp_dir.path(), toml::Value::Array(vec![]))?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "new", "--active", "Must not be created"]],
    )?;
    assert!(output.contains("error[E0505]"), "{output}");
    assert!(output.contains("govctl migrate"), "{output}");
    assert_eq!(fs::read_dir(temp_dir.path().join("gov/work"))?.count(), 0);
    Ok(())
}

#[test]
fn test_schema_v4_gate_precedes_full_config_load_but_not_project_independent_commands() -> TestResult
{
    let temp_dir = init_project()?;
    write_schema_four_source_scan(
        temp_dir.path(),
        toml::Value::String("not-an-array".to_string()),
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "--active", "Must not be created"],
            &["describe"],
            &["completions", "bash"],
        ],
    )?;
    assert_eq!(output.matches("error[E0505]").count(), 1, "{output}");
    assert!(!output.contains("Failed to parse config"), "{output}");
    assert!(output.contains("\"schema_version\": 1"), "{output}");
    assert!(output.contains("_govctl()"), "{output}");
    assert_eq!(fs::read_dir(temp_dir.path().join("gov/work"))?.count(), 0);
    Ok(())
}
