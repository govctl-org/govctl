//! Tests for source code reference scanning.
//!
//! Tests the scan_source_refs function which scans source files for
//! references to governance artifacts.

mod common;

use common::{init_project_with_date, run_commands, run_normalized_commands};
use std::fs;
use std::path::Path;

/// Helper to enable source scanning in a project.
/// Parses config as typed TOML, inserts an active `[source_scan]` table, and writes back.
fn enable_source_scan(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let content = fs::read_to_string(&config_path)?;
    let mut doc: toml::Table = toml::from_str(&content)?;
    let mut scan = toml::Table::new();
    scan.insert("enabled".into(), toml::Value::Boolean(true));
    scan.insert(
        "include".into(),
        toml::Value::Array(vec![toml::Value::String("src/**/*.rs".into())]),
    );
    doc.insert("source_scan".into(), toml::Value::Table(scan));
    fs::write(&config_path, toml::to_string_pretty(&doc)?)?;
    Ok(())
}

fn set_source_scan_include(
    dir: &Path,
    patterns: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(&config_path)?)?;
    config["source_scan"]["include"] = toml::Value::Array(
        patterns
            .iter()
            .map(|pattern| toml::Value::String((*pattern).to_string()))
            .collect(),
    );
    fs::write(config_path, toml::to_string_pretty(&config)?)?;
    Ok(())
}

fn set_source_scan_pattern(dir: &Path, pattern: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(&config_path)?)?;
    let config_table = config
        .as_table_mut()
        .ok_or("config root must be a TOML table")?;
    let source_scan = config_table
        .entry("source_scan")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or("source_scan must be a TOML table")?;
    source_scan.insert(
        "pattern".to_string(),
        toml::Value::String(pattern.to_string()),
    );
    fs::write(config_path, toml::to_string_pretty(&config)?)?;
    Ok(())
}

fn init_source_scan_project() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let (temp_dir, date) = init_project_with_date()?;
    enable_source_scan(temp_dir.path())?;
    Ok((temp_dir, date))
}

fn write_main_rs(dir: &Path, contents: impl AsRef<str>) -> Result<(), Box<dyn std::error::Error>> {
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), contents.as_ref())?;
    Ok(())
}

fn create_normative_rfc(dir: &Path, id: &str, title: &str) -> common::TestResult {
    run_commands(
        dir,
        &[
            &["rfc", "new", title],
            &["rfc", "finalize", id, "normative"],
        ],
    )?;
    Ok(())
}

macro_rules! assert_scan_check_snapshot {
    ($temp_dir:expr, $date:expr) => {{
        let value = run_normalized_commands($temp_dir.path(), $date, &[&["check"]])?;
        crate::assert_current_test_snapshot!("test_scan", value);
        Ok(())
    }};
}

#[test]
fn test_scan_no_references() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_disabled_does_not_read_ignore_files() -> common::TestResult {
    let (temp_dir, _) = init_project_with_date()?;
    fs::create_dir(temp_dir.path().join(".govignore"))?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains(".govignore"), "{output}");
    assert!(!output.contains("source files scanned"), "{output}");
    assert!(!output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_scan_empty_include_selects_no_files() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &[])?;
    write_main_rs(temp_dir.path(), "fn main() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  0 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_include_matching_is_case_sensitive() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("UPPER.RS"), "fn upper() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  0 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_valid_rfc_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Test RFC")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_uses_project_root_when_run_from_subdirectory() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Test RFC")?;
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    let docs_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&docs_dir)?;
    let output = run_commands(&docs_dir, &[&["check"]])?;

    assert!(
        output.contains("  1 source files scanned"),
        "output: {}",
        output
    );
    assert!(
        output.contains("  1 references found"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_scan_valid_clause_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

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
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001:C-TEST]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_unknown_rfc_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-9999]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_check_rejects_pattern_without_capture_group_when_scan_disabled() -> common::TestResult {
    let (temp_dir, _) = init_project_with_date()?;
    set_source_scan_pattern(temp_dir.path(), r"\[\[RFC-\d{4}\]\]")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(output.contains("capture group 1 is required"), "{output}");
    assert_eq!(output.matches("error[E0501]").count(), 1, "{output}");
    Ok(())
}

#[test]
fn test_check_reports_pattern_contract_once_when_scan_enabled() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_pattern(temp_dir.path(), r"\[\[RFC-\d{4}\]\]")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("capture group 1 is required"), "{output}");
    assert_eq!(output.matches("error[E0501]").count(), 1, "{output}");
    Ok(())
}

#[test]
fn test_check_reports_pattern_contract_when_project_loading_fails() -> common::TestResult {
    let (temp_dir, _) = init_project_with_date()?;
    set_source_scan_pattern(temp_dir.path(), r"\[\[RFC-\d{4}\]\]")?;
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(&rfc_dir)?;
    fs::write(rfc_dir.join("rfc.toml"), "not valid TOML =")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(output.contains("capture group 1 is required"), "{output}");
    assert!(output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_scan_rejects_match_without_participating_capture() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_pattern(temp_dir.path(), r"(?:PLAIN|\[\[([A-Z][A-Z0-9-]*)\]\])")?;
    write_main_rs(temp_dir.path(), "PLAIN\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(
        output.contains("capture group 1 did not participate"),
        "{output}"
    );
    assert!(output.contains("src/main.rs:1:1"), "{output}");
    assert!(output.contains("  0 references found"), "{output}");
    Ok(())
}

#[test]
fn test_scan_rejects_empty_capture() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_pattern(temp_dir.path(), r"\[\[([^]]*)\]\]")?;
    write_main_rs(temp_dir.path(), "[[]]\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(
        output.contains("capture group 1 matched an empty target"),
        "{output}"
    );
    assert!(output.contains("src/main.rs:1:3"), "{output}");
    assert!(output.contains("  0 references found"), "{output}");
    Ok(())
}

#[test]
fn test_scan_reports_one_based_byte_column() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    write_main_rs(temp_dir.path(), "// é [[RFC-9999]]\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("src/main.rs:1:9"), "{output}");
    Ok(())
}

#[test]
fn test_scan_orders_diagnostics_by_path_and_position() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    fs::write(temp_dir.path().join("z.rs"), "[[RFC-9003]]\n")?;
    fs::write(temp_dir.path().join("a.rs"), "[[RFC-9001]]\n[[RFC-9002]]\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    let first = output.find("a.rs:1:3").ok_or("missing first location")?;
    let second = output.find("a.rs:2:3").ok_or("missing second location")?;
    let third = output.find("z.rs:1:3").ok_or("missing third location")?;
    assert!(first < second && second < third, "{output}");
    Ok(())
}

#[test]
fn test_scan_unknown_clause_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Test RFC")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001:C-NONEXISTENT]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_deprecated_rfc_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Old RFC")?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "deprecate", "RFC-0001", "--force"]],
    )?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_valid_adr_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "accept", "ADR-0001"],
        ],
    )?;

    write_main_rs(temp_dir.path(), "// Follows [[ADR-0001]]\nfn main() {}\n")?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_valid_work_item_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    run_commands(temp_dir.path(), &[&["work", "new", "Test task"]])?;

    let wi_output = run_commands(temp_dir.path(), &[&["work", "list", "all"]])?;
    let wi_id = regex::Regex::new(r"WI-\d{4}-\d{2}-\d{2}-\d{3}")?
        .find(&wi_output)
        .ok_or("No work item ID found")?
        .as_str()
        .to_string();

    write_main_rs(
        temp_dir.path(),
        format!("// Implements [[{}]]\nfn main() {{}}\n", wi_id),
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_multiple_references_in_file() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "RFC One")?;
    create_normative_rfc(temp_dir.path(), "RFC-0002", "RFC Two")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]] and [[RFC-0002]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_mixed_valid_invalid_references() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Valid RFC")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]] and [[RFC-9999]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_govignore_reincludes_gitignored_file_and_parent() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    fs::write(temp_dir.path().join(".gitignore"), "ignored/\n")?;
    fs::write(
        temp_dir.path().join(".govignore"),
        "!ignored/\n!ignored/keep.rs\n",
    )?;
    fs::create_dir(temp_dir.path().join("ignored"))?;
    fs::write(temp_dir.path().join("ignored/keep.rs"), "fn keep() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  1 source files scanned"), "{output}");
    assert!(!output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_scan_deeper_govignore_overrides_root_rule() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    write_main_rs(temp_dir.path(), "fn main() {}\n")?;
    fs::write(temp_dir.path().join(".govignore"), "src/*.rs\n")?;
    fs::write(temp_dir.path().join("src/.govignore"), "!main.rs\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  1 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_last_matching_rule_wins_within_one_ignore_file() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    write_main_rs(temp_dir.path(), "fn main() {}\n")?;
    fs::write(
        temp_dir.path().join(".govignore"),
        "src/*.rs\n!src/main.rs\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  1 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_deeper_gitignore_overrides_root_rule() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    write_main_rs(temp_dir.path(), "fn main() {}\n")?;
    fs::write(temp_dir.path().join(".gitignore"), "src/*.rs\n")?;
    fs::write(temp_dir.path().join("src/.gitignore"), "!main.rs\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  1 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_root_govignore_overrides_deeper_gitignore() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    fs::write(
        temp_dir.path().join(".govignore"),
        "!nested/\n!nested/keep.rs\n",
    )?;
    let nested = temp_dir.path().join("nested");
    fs::create_dir(&nested)?;
    fs::write(nested.join(".gitignore"), "keep.rs\n")?;
    fs::write(nested.join("keep.rs"), "fn keep() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  1 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_deeper_govignore_overrides_root_gitignore() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    fs::write(
        temp_dir.path().join(".gitignore"),
        "!nested/\n!nested/skip.rs\n",
    )?;
    let nested = temp_dir.path().join("nested");
    fs::create_dir(&nested)?;
    fs::write(nested.join(".govignore"), "skip.rs\n")?;
    fs::write(nested.join("skip.rs"), "fn skip() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  0 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_prunes_ignored_directory_before_nested_ignore_file() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    fs::write(temp_dir.path().join(".govignore"), "excluded/\n")?;
    let excluded = temp_dir.path().join("excluded");
    fs::create_dir(&excluded)?;
    fs::write(excluded.join("hidden.rs"), "fn hidden() {}\n")?;
    fs::create_dir(excluded.join(".govignore"))?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  0 source files scanned"), "{output}");
    assert!(!output.contains("Source scan traversal failed"), "{output}");
    Ok(())
}

#[test]
fn test_scan_prunes_git_metadata_directory() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    write_main_rs(temp_dir.path(), "fn main() {}\n")?;
    let git_objects = temp_dir.path().join(".git/objects");
    fs::create_dir_all(&git_objects)?;
    fs::write(git_objects.join("metadata.rs"), "fn metadata() {}\n")?;
    fs::create_dir(git_objects.join(".govignore"))?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  1 source files scanned"), "{output}");
    assert!(!output.contains("Source scan traversal failed"), "{output}");
    Ok(())
}

#[test]
fn test_scan_reports_reached_invalid_ignore_file() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    fs::write(temp_dir.path().join(".govignore"), "[z-a]\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(
        output.contains("Invalid source scan ignore rule"),
        "{output}"
    );
    assert!(output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_scan_reports_reached_unreadable_ignore_path() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    fs::create_dir(temp_dir.path().join(".govignore"))?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0901]"), "{output}");
    assert!(output.contains(".govignore"), "{output}");
    assert!(output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_scan_reports_selected_source_decode_failure() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("invalid.rs"), [0xff, 0xfe])?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0901]"), "{output}");
    assert!(output.contains("read selected source file"), "{output}");
    assert!(output.contains("src/invalid.rs"), "{output}");
    assert!(output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_scan_rejects_invalid_positive_include_pattern() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["src/[z-a].rs"])?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(output.contains("source_scan.include"), "{output}");
    assert!(output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_scan_rejects_negated_positive_include_pattern() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["!src/main.rs"])?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0501]"), "{output}");
    assert!(output.contains("leading '!' is not allowed"), "{output}");
    Ok(())
}

#[test]
fn test_scan_applies_gitignore_style_positive_include_forms() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(
        temp_dir.path(),
        &["/root.rs", "nested.rs", "#hash.rs", "selected/"],
    )?;
    fs::write(temp_dir.path().join("root.rs"), "fn root() {}\n")?;
    fs::write(temp_dir.path().join("#hash.rs"), "fn hash() {}\n")?;
    fs::create_dir_all(temp_dir.path().join("deep"))?;
    fs::write(temp_dir.path().join("deep/nested.rs"), "fn nested() {}\n")?;
    fs::write(temp_dir.path().join("deep/root.rs"), "fn not_root() {}\n")?;
    fs::create_dir_all(temp_dir.path().join("selected/deeper"))?;
    fs::write(
        temp_dir.path().join("selected/deeper/data.txt"),
        "selected\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  4 source files scanned"), "{output}");
    Ok(())
}

#[test]
fn test_scan_does_not_use_git_info_exclude_or_hidden_filter() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    fs::create_dir_all(temp_dir.path().join(".git/info"))?;
    fs::write(temp_dir.path().join(".git/info/exclude"), "src/\n")?;
    fs::write(temp_dir.path().join(".ignore"), "src/\n")?;
    fs::write(temp_dir.path().join(".hidden.rs"), "fn hidden() {}\n")?;
    write_main_rs(temp_dir.path(), "fn main() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  2 source files scanned"), "{output}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_scan_does_not_follow_symbolic_links() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let (temp_dir, _) = init_source_scan_project()?;
    set_source_scan_include(temp_dir.path(), &["**/*.rs"])?;
    let external = tempfile::tempdir()?;
    fs::write(external.path().join("linked.rs"), "fn linked() {}\n")?;
    symlink(external.path(), temp_dir.path().join("linked"))?;
    write_main_rs(temp_dir.path(), "fn main() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("  1 source files scanned"), "{output}");
    Ok(())
}
