//! Tests for lifecycle commands - RFC/ADR status and phase transitions.
//!
//! Covers: finalize, advance, accept, reject, bump, deprecate, supersede.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

// ============================================================================
// RFC Finalize Tests
// ============================================================================

#[test]
fn test_finalize_draft_to_normative() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "list"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_draft_to_deprecated() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Obsolete RFC"],
            &["rfc", "finalize", "RFC-0001", "deprecated"],
            &["rfc", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_normative_to_deprecated() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001"],
            &["rfc", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_already_normative_fails() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-9999", "normative"]],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_legacy_json_rfc_requires_migrate() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(&rfc_dir)?;
    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Legacy RFC",
  "version": "0.1.0",
  "status": "draft",
  "phase": "spec",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [],
  "changelog": [{ "version": "0.1.0", "date": "2026-01-01", "notes": "Initial draft" }]
}"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "finalize", "RFC-0001", "normative"]],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// RFC Advance Tests
// ============================================================================

#[test]
fn test_advance_spec_to_impl() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_impl_to_test() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_test_to_stable() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_draft_to_impl_fails() -> common::TestResult {
    // Cannot advance draft RFC to impl phase
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "advance", "RFC-0001", "impl"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_skip_phase_fails() -> common::TestResult {
    // Cannot skip phases (e.g., spec -> test)
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "test"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_backwards_fails() -> common::TestResult {
    // Cannot go backwards (e.g., impl -> spec)
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "spec"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-9999", "impl"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_sets_updated_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Updated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "get", "RFC-0001", "updated"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_sets_updated_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Updated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "get", "RFC-0001", "updated"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_deprecate_legacy_json_clause_requires_migrate() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let clauses_dir = temp_dir.path().join("gov/rfc/RFC-0001/clauses");
    fs::create_dir_all(&clauses_dir)?;
    fs::write(
        clauses_dir.join("C-TEST.json"),
        r#"{
  "clause_id": "C-TEST",
  "title": "Legacy Clause",
  "kind": "normative",
  "status": "active",
  "text": "Legacy clause content."
}"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "deprecate", "RFC-0001:C-TEST", "--force"]],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// RFC Bump Tests
// ============================================================================

#[test]
fn test_bump_patch_version() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_minor_version() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_major_version() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_requires_summary() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "bump", "RFC-0001", "--patch"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_with_change() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "bump", "RFC-0001", "--change", "Added new clause"],
            &["rfc", "show", "RFC-0001"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_bump_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "bump", "RFC-9999", "--patch", "--summary", "Fix"]],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// ADR Accept/Reject Tests
// ============================================================================

#[test]
fn test_accept_proposed_adr() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            // Implements [[ADR-0042]]: must have alternatives before accepting
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &["adr", "add", "ADR-0001", "alternatives", "Option B"],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "0",
                "-s",
                "accepted",
            ],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "1",
                "-s",
                "rejected",
            ],
            &["adr", "list"],
            &["adr", "accept", "ADR-0001"],
            &["adr", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_reject_proposed_adr() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Bad Decision"],
            &["adr", "reject", "ADR-0001"],
            &["adr", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_accept_already_accepted_fails() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            // Implements [[ADR-0042]]: must have alternatives before accepting
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &["adr", "add", "ADR-0001", "alternatives", "Option B"],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "0",
                "-s",
                "accepted",
            ],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "1",
                "-s",
                "rejected",
            ],
            &["adr", "accept", "ADR-0001"],
            &["adr", "accept", "ADR-0001"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_accept_rejected_adr_fails() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Bad Decision"],
            &["adr", "reject", "ADR-0001"],
            &["adr", "accept", "ADR-0001"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_accept_nonexistent_adr() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["adr", "accept", "ADR-9999"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// RFC Deprecate/Supersede Tests
// ============================================================================

#[test]
fn test_deprecate_normative_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001", "--force"],
            &["rfc", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_supersede_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_supersede_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "New RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "supersede", "RFC-9999", "--by", "RFC-0001"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// Clause Supersede Tests
// ============================================================================

#[test]
fn test_supersede_clause() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_deprecate_clause_force() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Deprecate Clause RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-ONE",
                "Clause One",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "deprecate", "RFC-0001:C-ONE", "--force"],
            &["clause", "list"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_deprecate_clause_already_deprecated_fails() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Deprecate Twice RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-ONE",
                "Clause One",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "deprecate", "RFC-0001:C-ONE", "--force"],
            &["clause", "deprecate", "RFC-0001:C-ONE", "--force"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_deprecate_clause_superseded_fails() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Supersede Then Deprecate RFC"],
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
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-NEW",
                "--force",
            ],
            &["clause", "deprecate", "RFC-0001:C-OLD", "--force"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
