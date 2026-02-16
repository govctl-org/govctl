//! Tests for the describe command - machine-readable CLI metadata for agents.

mod common;

use common::{init_project, normalize_output, run_commands, today};

#[test]
fn test_describe_basic() {
    // describe without a project should output static metadata
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["describe"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_in_initialized_project() {
    // describe in an initialized project (without --context) should still output static metadata
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["describe"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_empty_project() {
    // describe --context in an empty initialized project should show empty state
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["describe", "--context"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_draft_rfc() {
    // describe --context with a draft RFC should suggest finalizing
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["describe", "--context"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_normative_spec_phase_rfc() {
    // describe --context with a normative RFC in spec phase should suggest advancing to impl
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["describe", "--context"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_normative_impl_phase_rfc() {
    // describe --context with a normative RFC in impl phase should suggest advancing to test
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["describe", "--context"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_normative_test_phase_rfc() {
    // describe --context with a normative RFC in test phase should suggest advancing to stable
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["describe", "--context"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_proposed_adr() {
    // describe --context with a proposed ADR should suggest accepting
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["describe", "--context"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_active_work_item() {
    // describe --context with an active work item should show it
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test task", "--active"],
            &["describe", "--context"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_describe_with_context_queued_work_items() {
    // describe --context with queued work items but no active should suggest activating one
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Task one"],
            &["work", "new", "Task two"],
            &["describe", "--context"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
