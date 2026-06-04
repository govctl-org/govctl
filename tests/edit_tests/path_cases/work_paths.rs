use super::*;

#[test]
fn test_work_get_nested_scalar_rejects_index() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let wi_id = format!("WI-{}-001", date);
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Work Scalar Index Test"],
            &[
                "work",
                "add",
                &wi_id,
                "acceptance_criteria",
                "add: Did something",
            ],
            &["work", "get", &wi_id, "acceptance_criteria[0].text[0]"],
        ],
    )?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    assert!(
        normalized.contains("error[E0817]: Cannot index into non-list field 'text'"),
        "output: {normalized}"
    );
    Ok(())
}

#[test]
fn test_work_journal_legacy_entries_are_read_only() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let wi_id = format!("WI-{}-001", date);
    let work_path = temp_dir
        .path()
        .join("gov")
        .join("work")
        .join(format!("{}-legacy-journal-test.toml", date));
    let output = run_commands(temp_dir.path(), &[&["work", "new", "Legacy Journal Test"]])?;
    assert!(output.contains("Created work item"), "output: {output}");

    let mut content = std::fs::read_to_string(&work_path)?;
    content.push_str(
        r#"

[[content.journal]]
date = "2026-02-22"
content = "Keep this history"
"#,
    );
    std::fs::write(&work_path, content)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "get", &wi_id, "journal[0].content"],
            &[
                "work",
                "edit",
                &wi_id,
                "journal[0].content",
                "--set",
                "Changed history",
            ],
            &["work", "remove", &wi_id, "journal", "--all"],
            &["work", "show", &wi_id],
        ],
    )?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    assert!(
        normalized.contains("Keep this history"),
        "legacy journal should still render via work show: {normalized}"
    );
    assert_eq!(
        normalized.matches("error[E0803]").count(),
        3,
        "legacy journal get/set/remove field operations should be rejected: {normalized}"
    );
    assert!(
        normalized.contains("## Journal")
            && normalized.contains("Legacy execution history")
            && normalized.contains("loop state"),
        "legacy journal should render with neutral migration guidance: {normalized}"
    );
    let persisted = std::fs::read_to_string(&work_path)?;
    assert!(
        !persisted.contains("Changed history"),
        "legacy journal should not be mutated: {persisted}"
    );
    Ok(())
}
