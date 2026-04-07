//! Snapshot tests for key CLI help contracts.

mod common;

use common::{normalize_output, run_commands, today};

#[test]
fn test_rfc_get_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["rfc", "get", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_rfc_root_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["rfc", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_get_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["adr", "get", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_root_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["adr", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_get_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["work", "get", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_root_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["work", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_clause_root_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["clause", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_guard_root_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["guard", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_tick_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["adr", "tick", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_tick_help() {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["work", "tick", "--help"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
