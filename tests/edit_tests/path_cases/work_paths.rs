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
                "edit",
                &wi_id,
                "acceptance_criteria",
                "--add",
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
fn test_work_journal_storage_is_rejected() -> common::TestResult {
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
            &["check"],
            &["work", "get", &wi_id, "journal[0].content"],
            &[
                "work",
                "edit",
                &wi_id,
                "journal[0].content",
                "--set",
                "Changed history",
            ],
            &["work", "edit", &wi_id, "journal", "--remove", "--all"],
            &["work", "show", &wi_id],
        ],
    )?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    assert!(
        normalized.contains("Additional properties are not allowed"),
        "legacy journal storage should be rejected: {normalized}"
    );
    assert!(
        !normalized.contains("## Journal"),
        "legacy journal must not render: {normalized}"
    );
    let persisted = std::fs::read_to_string(&work_path)?;
    assert!(
        !persisted.contains("Changed history"),
        "legacy journal should not be mutated: {persisted}"
    );
    Ok(())
}
