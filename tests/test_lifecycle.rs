//! Tests for lifecycle commands - RFC/ADR status and phase transitions.
//!
//! Covers: finalize, advance, accept, reject, bump, deprecate, supersede.

mod common;

use common::{init_project, normalize_output, run_commands, today};

// ============================================================================
// RFC Finalize Tests
// ============================================================================

#[test]
fn test_finalize_draft_to_normative() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "list"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_finalize_draft_to_deprecated() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Obsolete RFC"],
            &["rfc", "finalize", "RFC-0001", "deprecated"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_finalize_normative_to_deprecated() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_finalize_already_normative_fails() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_finalize_nonexistent_rfc() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-9999", "normative"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// RFC Advance Tests
// ============================================================================

#[test]
fn test_advance_spec_to_impl() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_advance_impl_to_test() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_advance_test_to_stable() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_advance_draft_to_impl_fails() {
    // Cannot advance draft RFC to impl phase
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "advance", "RFC-0001", "impl"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_advance_skip_phase_fails() {
    // Cannot skip phases (e.g., spec -> test)
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "test"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_advance_backwards_fails() {
    // Cannot go backwards (e.g., impl -> spec)
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "spec"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// RFC Bump Tests
// ============================================================================

#[test]
fn test_bump_patch_version() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Minor fix",
            ],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_bump_minor_version() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--minor",
                "--summary",
                "New feature",
            ],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_bump_major_version() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--major",
                "--summary",
                "Breaking change",
            ],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_bump_requires_summary() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "bump", "RFC-0001", "--patch"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_bump_with_change() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "bump", "RFC-0001", "--change", "Added new clause"],
            &["rfc", "show", "RFC-0001"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_bump_nonexistent_rfc() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "bump", "RFC-9999", "--patch", "--summary", "Fix"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// ADR Accept/Reject Tests
// ============================================================================

#[test]
fn test_accept_proposed_adr() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "list"],
            &["adr", "accept", "ADR-0001"],
            &["adr", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_reject_proposed_adr() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Bad Decision"],
            &["adr", "reject", "ADR-0001"],
            &["adr", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_accept_already_accepted_fails() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "accept", "ADR-0001"],
            &["adr", "accept", "ADR-0001"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_accept_rejected_adr_fails() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Bad Decision"],
            &["adr", "reject", "ADR-0001"],
            &["adr", "accept", "ADR-0001"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_accept_nonexistent_adr() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["adr", "accept", "ADR-9999"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// RFC Deprecate/Supersede Tests
// ============================================================================

#[test]
fn test_deprecate_normative_rfc() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_supersede_rfc() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &["rfc", "new", "New RFC"],
            &["rfc", "finalize", "RFC-0002", "normative"],
            &["rfc", "supersede", "RFC-0001", "--by", "RFC-0002"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_supersede_nonexistent_rfc() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "New RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "supersede", "RFC-9999", "--by", "RFC-0001"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// Clause Supersede Tests
// ============================================================================

#[test]
fn test_supersede_clause() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-OLD",
                "Old Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "new",
                "RFC-0001:C-NEW",
                "New Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-NEW",
            ],
            &["clause", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
