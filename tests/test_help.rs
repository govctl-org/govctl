//! Snapshot tests for key CLI help contracts.

mod common;

use common::{normalize_output, run_commands};

macro_rules! assert_help_snapshot {
    ($command:expr) => {{
        let (temp_dir, date) = common::temp_dir_with_date()?;
        let output = run_commands(temp_dir.path(), &[$command])?;
        let normalized = normalize_output(&output, temp_dir.path(), &date)?;
        crate::assert_current_test_snapshot!("test_help", normalized);
        Ok(())
    }};
}

#[test]
fn test_rfc_get_help() -> common::TestResult {
    assert_help_snapshot!(&["rfc", "get", "--help"])
}

#[test]
fn test_rfc_root_help() -> common::TestResult {
    assert_help_snapshot!(&["rfc", "--help"])
}

#[test]
fn test_rfc_edit_help() -> common::TestResult {
    assert_help_snapshot!(&["rfc", "edit", "--help"])
}

#[test]
fn test_rfc_bump_help() -> common::TestResult {
    assert_help_snapshot!(&["rfc", "bump", "--help"])
}

#[test]
fn test_adr_get_help() -> common::TestResult {
    assert_help_snapshot!(&["adr", "get", "--help"])
}

#[test]
fn test_adr_root_help() -> common::TestResult {
    assert_help_snapshot!(&["adr", "--help"])
}

#[test]
fn test_work_get_help() -> common::TestResult {
    assert_help_snapshot!(&["work", "get", "--help"])
}

#[test]
fn test_work_root_help() -> common::TestResult {
    assert_help_snapshot!(&["work", "--help"])
}

#[test]
fn test_loop_root_help() -> common::TestResult {
    assert_help_snapshot!(&["loop", "--help"])
}

#[test]
fn test_clause_root_help() -> common::TestResult {
    assert_help_snapshot!(&["clause", "--help"])
}

#[test]
fn test_clause_edit_help() -> common::TestResult {
    assert_help_snapshot!(&["clause", "edit", "--help"])
}

#[test]
fn test_guard_root_help() -> common::TestResult {
    assert_help_snapshot!(&["guard", "--help"])
}

#[test]
fn test_adr_tick_help() -> common::TestResult {
    assert_help_snapshot!(&["adr", "tick", "--help"])
}

#[test]
fn test_work_tick_help() -> common::TestResult {
    assert_help_snapshot!(&["work", "tick", "--help"])
}
