//! Tests for the describe command - machine-readable CLI metadata for agents.

mod common;

use common::{init_project_with_date, run_normalized_commands, temp_dir_with_date};

#[test]
fn test_describe_basic() -> common::TestResult {
    let (temp_dir, date) = temp_dir_with_date()?;

    // Expected: static metadata is available without an initialized project.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[&["describe"]]
    )?);
    Ok(())
}

#[test]
fn test_describe_in_initialized_project() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected: plain describe still returns static metadata inside a project.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[&["describe"]]
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_empty_project() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected: context output reports an empty initialized project state.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[&["describe", "--context"]]
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_draft_rfc() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected context action: finalize the draft RFC.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[&["rfc", "new", "Test RFC"], &["describe", "--context"]],
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_normative_spec_phase_rfc() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected context action: advance the RFC from spec to impl.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["describe", "--context"],
        ],
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_normative_impl_phase_rfc() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected context action: advance the RFC from impl to test.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["describe", "--context"],
        ],
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_normative_test_phase_rfc() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected context action: advance the RFC from test to stable.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["describe", "--context"],
        ],
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_proposed_adr() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected context action: accept the proposed ADR.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[&["adr", "new", "Test Decision"], &["describe", "--context"]],
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_active_work_item() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected context state: show the active work item.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[
            &["work", "new", "Test task", "--active"],
            &["describe", "--context"],
        ],
    )?);
    Ok(())
}

#[test]
fn test_describe_with_context_queued_work_items() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Expected context action: activate one queued work item.
    insta::assert_snapshot!(run_normalized_commands(
        temp_dir.path(),
        &date,
        &[
            &["work", "new", "Task one"],
            &["work", "new", "Task two"],
            &["describe", "--context"],
        ],
    )?);
    Ok(())
}
