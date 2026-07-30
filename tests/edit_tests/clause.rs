use super::*;

// ============================================================================
// Clause Edit Tests
// ============================================================================

const NEW_TEST_RFC: &[&str] = &["rfc", "new", "Test RFC"];
const TEST_CLAUSE_ID: &str = "RFC-0001:C-TEST";
const SHOW_TEST_CLAUSE: &[&str] = &["clause", "show", TEST_CLAUSE_ID];

fn new_test_clause(title: &'static str) -> [&'static str; 8] {
    [
        "clause",
        "new",
        TEST_CLAUSE_ID,
        title,
        "-s",
        "Specification",
        "-k",
        "normative",
    ]
}

#[test]
fn test_clause_set_text() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let clause = new_test_clause("Test Clause");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "text",
                "--set",
                "Updated clause text",
            ],
            SHOW_TEST_CLAUSE,
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_edit_text_canonical() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let clause = new_test_clause("Test Clause");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "text",
                "--set",
                "Updated clause text",
            ],
            SHOW_TEST_CLAUSE,
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_edit_rejects_schema_invalid_text_without_writing() -> common::TestResult {
    let temp_dir = init_project()?;
    let clause = new_test_clause("Test Clause");
    run_commands(temp_dir.path(), &[NEW_TEST_RFC, &clause])?;
    let clause_path = temp_dir.path().join("gov/rfc/RFC-0001/clauses/C-TEST.toml");
    let before = std::fs::read(&clause_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "edit", TEST_CLAUSE_ID, "text", "--set", ""]],
    )?;

    assert!(output.contains("error[E0201]"), "{output}");
    assert!(output.contains("is shorter than 1 character"), "{output}");
    assert_eq!(std::fs::read(clause_path)?, before);
    Ok(())
}

#[test]
fn test_clause_set_title() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let clause = new_test_clause("Original Title");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "title",
                "--set",
                "New Title",
            ],
            SHOW_TEST_CLAUSE,
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_edit_title_canonical() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let clause = new_test_clause("Original Title");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "title",
                "--set",
                "New Title",
            ],
            SHOW_TEST_CLAUSE,
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_remove_anchor_by_index_canonical() -> common::TestResult {
    let temp_dir = init_project()?;
    let clause = new_test_clause("Original Title");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "anchors",
                "--add",
                "anchor-one",
            ],
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "anchors",
                "--add",
                "anchor-two",
            ],
            &["clause", "edit", TEST_CLAUSE_ID, "anchors[0]", "--remove"],
            &["clause", "show", TEST_CLAUSE_ID, "-o", "json"],
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
    let (temp_dir, date) = init_project_with_date()?;
    let clause = new_test_clause("Test Clause");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &["clause", "get", TEST_CLAUSE_ID, "title"],
            &["clause", "get", TEST_CLAUSE_ID, "kind"],
            &["clause", "get", TEST_CLAUSE_ID, "status"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_set_since_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    let clause = new_test_clause("Test Clause");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &["clause", "edit", TEST_CLAUSE_ID, "since", "--set", "0.1.0"],
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
fn test_clause_edit_text() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let clause = new_test_clause("Test Clause");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "text",
                "--set",
                "new text",
            ],
            SHOW_TEST_CLAUSE,
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_clause_set_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    let clause = new_test_clause("Test Clause");

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &clause,
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "status",
                "--set",
                "deprecated",
            ],
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
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            NEW_TEST_RFC,
            &[
                "clause",
                "edit",
                "RFC-0001:C-NONEXISTENT",
                "text",
                "--set",
                "Text",
            ],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_clause_edit_rejects_clause_symlink_outside_rfc() -> common::TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = init_project()?;
    let clause = new_test_clause("Test Clause");
    run_commands(temp_dir.path(), &[NEW_TEST_RFC, &clause])?;
    let clause_path = temp_dir.path().join("gov/rfc/RFC-0001/clauses/C-TEST.toml");
    let external_path = temp_dir.path().join("external-clause.toml");
    std::fs::rename(&clause_path, &external_path)?;
    symlink(&external_path, &clause_path)?;
    let before = std::fs::read(&external_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["clause", "show", TEST_CLAUSE_ID],
            &[
                "clause",
                "edit",
                TEST_CLAUSE_ID,
                "text",
                "--set",
                "Redirected",
            ],
        ],
    )?;

    assert_eq!(output.matches("error[E0204]").count(), 2, "{output}");
    assert_eq!(std::fs::read(&external_path)?, before);
    Ok(())
}
