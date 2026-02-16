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
