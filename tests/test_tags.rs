//! Integration tests for the govctl tags feature.
//!
//! Covers: tag registry management, artifact tagging, validation, and list filtering.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

// ============================================================================
// Helper
// ============================================================================

/// Register allowed tags in config.toml by editing the TOML table directly.
fn register_tags(dir: &std::path::Path, tags: &[&str]) {
    let config_path = dir.join("gov/config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    let mut doc: toml::Table = toml::from_str(&content).unwrap();
    let arr: toml::value::Array = tags
        .iter()
        .map(|t| toml::Value::String(t.to_string()))
        .collect();
    let mut tags_table = toml::Table::new();
    tags_table.insert("allowed".into(), toml::Value::Array(arr));
    doc.insert("tags".into(), toml::Value::Table(tags_table));
    fs::write(&config_path, toml::to_string_pretty(&doc).unwrap()).unwrap();
}

// ============================================================================
// Registry management
// ============================================================================

#[test]
fn test_tag_new() {
    let temp_dir = init_project();
    let date = today();
    let output = run_commands(
        temp_dir.path(),
        &[&["tag", "new", "caching"], &["tag", "list"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_tag_new_duplicate() {
    let temp_dir = init_project();
    let date = today();
    let output = run_commands(
        temp_dir.path(),
        &[&["tag", "new", "caching"], &["tag", "new", "caching"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_tag_new_invalid_format() {
    let temp_dir = init_project();
    let date = today();
    let output = run_commands(temp_dir.path(), &[&["tag", "new", "UPPER"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_tag_delete() {
    let temp_dir = init_project();
    let date = today();
    let output = run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["tag", "delete", "caching"],
            &["tag", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_tag_delete_referenced() {
    let temp_dir = init_project();
    let date = today();
    let output = run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
            &["tag", "delete", "caching"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// Artifact tagging
// ============================================================================

#[test]
fn test_artifact_add_tag() {
    let temp_dir = init_project();
    let date = today();
    let output = run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
            &["adr", "get", "ADR-0001", "tags"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_artifact_add_unregistered_tag() {
    let temp_dir = init_project();
    let date = today();

    register_tags(temp_dir.path(), &["registered"]);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "nonexistent"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// Validation
// ============================================================================

#[test]
fn test_check_rejects_unknown_tag() {
    let temp_dir = init_project();
    let date = today();

    register_tags(temp_dir.path(), &["allowed-tag"]);

    run_commands(temp_dir.path(), &[&["adr", "new", "Test Decision"]]);

    // Find the ADR file and inject an unregistered tag
    let adr_dir = temp_dir.path().join("gov/adr");
    let adr_path = fs::read_dir(&adr_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("ADR-0001") && n.ends_with(".toml"))
                .unwrap_or(false)
        })
        .expect("ADR-0001 file not found in gov/adr");

    let content = fs::read_to_string(&adr_path).unwrap();
    let mut doc: toml::Table = toml::from_str(&content).unwrap();
    let govctl = doc
        .get_mut("govctl")
        .and_then(|v| v.as_table_mut())
        .expect("[govctl] table must exist in ADR TOML");
    govctl.insert(
        "tags".into(),
        toml::Value::Array(vec![toml::Value::String("unknown-tag".into())]),
    );
    fs::write(&adr_path, toml::to_string_pretty(&doc).unwrap()).unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_check_accepts_registered_tag() {
    let temp_dir = init_project();
    let date = today();

    register_tags(temp_dir.path(), &["caching"]);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
            &["check"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// List filtering
// ============================================================================

#[test]
fn test_list_filter_by_tag() {
    let temp_dir = init_project();
    let date = today();

    run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["adr", "new", "Tagged Decision"],
            &["adr", "new", "Untagged Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
        ],
    );

    let output = run_commands(temp_dir.path(), &[&["adr", "list", "--tag", "caching"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_list_filter_multiple_tags() {
    let temp_dir = init_project();
    let date = today();

    run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["tag", "new", "performance"],
            &["tag", "new", "security"],
            &["adr", "new", "Multi-Tagged Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
            &["adr", "add", "ADR-0001", "tags", "performance"],
        ],
    );

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "list", "--tag", "caching,performance"],
            &["adr", "list", "--tag", "caching,security"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
