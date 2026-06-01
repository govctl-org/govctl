use super::*;

// ============================================================================
// Clause Edit Tests
// ============================================================================

#[test]
fn test_clause_set_text() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_edit_text_canonical() -> common::TestResult {
    let temp_dir = init_project()?;
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
                "text",
                "--set",
                "Updated clause text",
            ],
            &["clause", "show", "RFC-0001:C-TEST"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_set_title() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_edit_title_canonical() -> common::TestResult {
    let temp_dir = init_project()?;
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
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "title",
                "--set",
                "New Title",
            ],
            &["clause", "show", "RFC-0001:C-TEST"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_remove_anchor_by_index_canonical() -> common::TestResult {
    let temp_dir = init_project()?;

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
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "anchors",
                "--add",
                "anchor-one",
            ],
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "anchors",
                "--add",
                "anchor-two",
            ],
            &[
                "clause",
                "edit",
                "RFC-0001:C-TEST",
                "anchors[0]",
                "--remove",
            ],
            &["clause", "show", "RFC-0001:C-TEST", "-o", "json"],
        ],
    )?;

    assert!(
        output.contains("Removed 'anchor-one' from RFC-0001:C-TEST.anchors"),
        "output: {}",
        output
    );
    assert!(
        output.contains("\"anchors\": [\n    \"anchor-two\"\n  ]")
            || output.contains("\"anchors\": [\n  \"anchor-two\"\n]"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_clause_get_field() -> common::TestResult {
    let temp_dir = init_project()?;
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
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_set_since_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

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
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(
        output.contains("Clause 'since' is derived from RFC versioning"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_clause_set_text_sugar() -> common::TestResult {
    let temp_dir = init_project()?;
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
            &["clause", "set", "RFC-0001:C-TEST", "text", "new text"],
            &["clause", "show", "RFC-0001:C-TEST"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_set_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;

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
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(
        output.contains("govctl clause deprecate"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_clause_edit_nonexistent() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["clause", "edit", "RFC-0001:C-NONEXISTENT", "--text", "Text"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
