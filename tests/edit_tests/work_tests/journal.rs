#[test]
fn test_work_journal_field_surface_is_unavailable() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let wi_id = format!("WI-{date}-001");
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "add", &wi_id, "journal", "First progress update"],
            &["work", "get", &wi_id, "journal"],
            &[
                "work",
                "edit",
                &wi_id,
                "journal",
                "--add",
                "Second progress update",
            ],
            &[
                "work",
                "edit",
                &wi_id,
                "journal[0].content",
                "--set",
                "Changed history",
            ],
            &["work", "remove", &wi_id, "journal", "--all"],
            &["work", "tick", &wi_id, "journal", "--at", "0"],
            &["work", "show", &wi_id],
        ],
    )?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    assert!(
        normalized.contains("Unknown work item field: journal"),
        "output: {normalized}"
    );
    assert_eq!(
        normalized.matches("error[E0803]").count(),
        6,
        "all journal field-surface operations should be rejected: {normalized}"
    );
    assert!(
        !normalized.contains("## Journal"),
        "journal write should not persist: {normalized}"
    );
    Ok(())
}

#[test]
fn test_work_new_omits_journal_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["work", "new", "Test Task"]])?;
    assert!(output.contains("Created work item"), "output: {output}");

    let work_path = temp_dir
        .path()
        .join("gov")
        .join("work")
        .join(format!("{date}-test-task.toml"));
    let content = std::fs::read_to_string(work_path)?;

    assert!(
        !content.contains("journal") && !content.contains("[[content.journal]]"),
        "new work items should omit journal: {content}"
    );
    Ok(())
}
