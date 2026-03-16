//! Tests for edit commands - modifying artifact fields.

mod common;

use common::{init_project, normalize_output, run_commands, today};

// ============================================================================
// RFC Field Edit Tests
// ============================================================================

#[test]
fn test_rfc_set_title() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Original Title"],
            &["rfc", "set", "RFC-0001", "title", "New Title"],
            &["rfc", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_rfc_get_field() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "get", "RFC-0001", "title"],
            &["rfc", "get", "RFC-0001", "status"],
            &["rfc", "get", "RFC-0001", "phase"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_rfc_add_owner() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@newowner"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_rfc_remove_owner() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "owners", "@owner1"],
            &["rfc", "add", "RFC-0001", "owners", "@owner2"],
            &["rfc", "remove", "RFC-0001", "owners", "@owner1"],
            &["rfc", "get", "RFC-0001", "owners"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_rfc_add_ref() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "add", "RFC-0001", "refs", "ADR-0001"],
            &["rfc", "get", "RFC-0001", "refs"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_rfc_set_nonexistent_field() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "nonexistent", "value"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_rfc_set_version_rejected() {
    let temp_dir = init_project();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "version", "0.2.0"],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(
        output.contains("Use `govctl rfc bump`"),
        "output: {}",
        output
    );
}

#[test]
fn test_rfc_set_status_rejected() {
    let temp_dir = init_project();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "set", "RFC-0001", "status", "normative"],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl rfc finalize"), "output: {}", output);
}

#[test]
fn test_rfc_get_nonexistent() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["rfc", "get", "RFC-9999", "title"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// Clause Edit Tests
// ============================================================================

#[test]
fn test_clause_set_text() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
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
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "--text",
                "Updated clause text",
            ],
            &["clause", "show", "RFC-0001:C-TEST"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_clause_set_title() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-TEST",
                "Original Title",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "set", "RFC-0001:C-TEST", "title", "New Title"],
            &["clause", "show", "RFC-0001:C-TEST"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_clause_get_field() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
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
            &["clause", "get", "RFC-0001:C-TEST", "title"],
            &["clause", "get", "RFC-0001:C-TEST", "kind"],
            &["clause", "get", "RFC-0001:C-TEST", "status"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_clause_set_since_rejected() {
    let temp_dir = init_project();

    let output = run_commands(
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
            &["clause", "set", "RFC-0001:C-TEST", "since", "0.1.0"],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(
        output.contains("Clause 'since' is derived from RFC versioning"),
        "output: {}",
        output
    );
}

#[test]
fn test_clause_set_text_rejected() {
    let temp_dir = init_project();

    let output = run_commands(
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
            &["clause", "set", "RFC-0001:C-TEST", "text", "new text"],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl clause edit"), "output: {}", output);
}

#[test]
fn test_clause_set_status_rejected() {
    let temp_dir = init_project();

    let output = run_commands(
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
            &["clause", "set", "RFC-0001:C-TEST", "status", "deprecated"],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(
        output.contains("govctl clause deprecate"),
        "output: {}",
        output
    );
}

#[test]
fn test_clause_edit_nonexistent() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["clause", "edit", "RFC-0001:C-NONEXISTENT", "--text", "Text"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// ADR Field Edit Tests
// ============================================================================

#[test]
fn test_adr_get_field() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "get", "ADR-0001", "title"],
            &["adr", "get", "ADR-0001", "status"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_set_title() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Original Title"],
            &["adr", "set", "ADR-0001", "title", "New Title"],
            &["adr", "list"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_set_status_rejected() {
    let temp_dir = init_project();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "set", "ADR-0001", "status", "accepted"],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl adr accept"), "output: {}", output);
}

#[test]
fn test_adr_set_alternative_status_rejected() {
    let temp_dir = init_project();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &[
                "adr",
                "set",
                "ADR-0001",
                "alternatives[0].status",
                "accepted",
            ],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl adr tick"), "output: {}", output);
}

#[test]
fn test_adr_add_ref() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "refs", "RFC-0001"],
            &["adr", "get", "ADR-0001", "refs"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_set_context() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &[
                "adr",
                "set",
                "ADR-0001",
                "context",
                "New context for the decision",
            ],
            &["adr", "show", "ADR-0001"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_set_decision() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "set", "ADR-0001", "decision", "We decided to do X"],
            &["adr", "show", "ADR-0001"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_set_consequences() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &[
                "adr",
                "set",
                "ADR-0001",
                "consequences",
                "Good: faster. Bad: more memory.",
            ],
            &["adr", "show", "ADR-0001"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_get_nonexistent() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["adr", "get", "ADR-9999", "title"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// Work Item Field Edit Tests
// ============================================================================

#[test]
fn test_work_get_field() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "get", &format!("WI-{}-001", date), "title"],
            &["work", "get", &format!("WI-{}-001", date), "status"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_set_title() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Original Title"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "title",
                "New Title",
            ],
            &["work", "list", "all"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_set_status_rejected() {
    let temp_dir = init_project();
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "set", &work_id, "status", "active"],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work move"), "output: {}", output);
}

#[test]
fn test_work_set_acceptance_criteria_status_rejected() {
    let temp_dir = init_project();
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &work_id,
                "acceptance_criteria",
                "add: Test criterion",
            ],
            &[
                "work",
                "set",
                &work_id,
                "acceptance_criteria[0].status",
                "done",
            ],
        ],
    );
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work tick"), "output: {}", output);
}

#[test]
fn test_work_add_acceptance_criteria() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 2",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_tick_acceptance_criteria() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 2",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "Criterion 1",
                "-s",
                "done",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_tick_cancel_acceptance_criteria() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "Criterion 1",
                "-s",
                "cancelled",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_add_journal() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "journal",
                "First progress update",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "journal",
                "Second progress update",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_add_ref() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "refs",
                "RFC-0001",
            ],
            &["work", "get", &format!("WI-{}-001", date), "refs"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_remove_acceptance_criteria() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: To remove",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: To keep",
            ],
            &[
                "work",
                "remove",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "To remove",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_get_nonexistent() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "get", "WI-9999-99-999", "title"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// Field Alias Tests
// ============================================================================

#[test]
fn test_field_alias_ac() {
    // 'ac' should resolve to 'acceptance_criteria'
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "ac",
                "add: Test criterion",
            ],
            &["work", "get", &format!("WI-{}-001", date), "ac"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_field_alias_desc() {
    // 'desc' should resolve to 'description'
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "desc",
                "A description",
            ],
            &["work", "get", &format!("WI-{}-001", date), "desc"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_field_alias_desc_under_legacy_prefix() {
    // content.desc should resolve to description on work items
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "content.desc",
                "Legacy-prefixed description",
            ],
            &["work", "get", &format!("WI-{}-001", date), "description"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_field_alias_desc_not_global_on_adr() {
    // desc is not a valid ADR root field alias and should not be rewritten globally
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Alias Scope"],
            &["adr", "set", "ADR-0001", "desc", "nope"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_tick_rejects_nested_path() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Nested Tick"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "ac[0].text",
                "Criterion 1",
                "-s",
                "done",
            ],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// ============================================================================
// Nested Path Tests (ADR-0029)
// ============================================================================

#[test]
fn test_adr_get_nested_path() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Path Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Use traits",
                "--pro",
                "Flexible",
                "--pro",
                "Reusable",
                "--con",
                "Complex",
            ],
            &["adr", "get", "ADR-0001", "alt[0].text"],
            &["adr", "get", "ADR-0001", "alt[0].pros"],
            &["adr", "get", "ADR-0001", "alt[0].pros[0]"],
            &["adr", "get", "ADR-0001", "alt[0].cons"],
            &["adr", "get", "ADR-0001", "alternatives[0]"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_set_nested_path() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Set Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option A",
                "--pro",
                "Fast",
                "--con",
                "Fragile",
            ],
            &["adr", "set", "ADR-0001", "alt[0].text", "Option A Revised"],
            &["adr", "get", "ADR-0001", "alt[0].text"],
            &["adr", "set", "ADR-0001", "alt[0].pros[0]", "Very fast"],
            &["adr", "get", "ADR-0001", "alt[0].pros[0]"],
            &[
                "adr",
                "set",
                "ADR-0001",
                "alt[0].rejection_reason",
                "Superseded by Option B",
            ],
            &["adr", "get", "ADR-0001", "alt[0].rejection_reason"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_add_nested_path() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Add Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Cheap",
            ],
            &["adr", "add", "ADR-0001", "alt[0].pros", "Reliable"],
            &["adr", "get", "ADR-0001", "alt[0].pros"],
            &["adr", "add", "ADR-0001", "alt[0].cons", "Slow"],
            &["adr", "get", "ADR-0001", "alt[0].cons"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_nested_path_rejects_extra_segments() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Depth Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Fast",
            ],
            &["adr", "get", "ADR-0001", "alt[0].pros[0].oops"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_add_nested_path_rejects_indexed_terminal() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Indexed Add Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Fast",
            ],
            &["adr", "add", "ADR-0001", "alt[0].pros[999]", "Ignored"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_get_nested_scalar_rejects_index() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Scalar Index Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Fast",
            ],
            &["adr", "get", "ADR-0001", "alt[0].text[0]"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_remove_nested_path() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Remove Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Opt1",
                "--pro",
                "Good",
                "--pro",
                "Great",
                "--con",
                "Bad",
            ],
            // Remove by sub-index
            &["adr", "remove", "ADR-0001", "alt[0].pros[0]"],
            &["adr", "get", "ADR-0001", "alt[0].pros"],
            // Remove con by pattern match (no terminal index)
            &["adr", "remove", "ADR-0001", "alt[0].cons", "Bad"],
            &["adr", "get", "ADR-0001", "alt[0].cons"],
            // Remove entire alternative
            &["adr", "remove", "ADR-0001", "alt[0]"],
            &["adr", "get", "ADR-0001", "alternatives"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_work_get_nested_scalar_rejects_index() {
    let temp_dir = init_project();
    let date = today();

    let wi_id = format!("WI-{}-001", date);
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Work Scalar Index Test"],
            &["work", "add", &wi_id, "journal", "Did something"],
            &["work", "get", &wi_id, "journal[0].content[0]"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_adr_remove_nested_path_requires_selector() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Selector Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Opt1",
                "--con",
                "Bad",
            ],
            &["adr", "remove", "ADR-0001", "alt[0].cons"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_remove_indexed_path_conflict() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Conflict Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Opt1",
                "--con",
                "Bad",
            ],
            // Indexed path + --exact should produce E0818
            &[
                "adr",
                "remove",
                "ADR-0001",
                "alt[0].cons[0]",
                "--exact",
                "Bad",
            ],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_path_backward_compat() {
    let temp_dir = init_project();
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Compat Test"],
            // Legacy dotted paths should still work
            &[
                "adr",
                "set",
                "ADR-0001",
                "content.decision",
                "A dotted decision",
            ],
            &["adr", "get", "ADR-0001", "content.decision"],
            &["adr", "set", "ADR-0001", "govctl.title", "Compat Title"],
            &["adr", "get", "ADR-0001", "govctl.title"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
