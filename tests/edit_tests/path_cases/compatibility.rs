use super::*;

#[test]
fn test_storage_prefixed_paths_are_rejected() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Compat Test"],
            // Implements [[ADR-0042]]: must have alternatives before setting decision
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--add",
                "Option A",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--add",
                "Option B",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives[0]",
                "--tick",
                "accepted",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives[1]",
                "--tick",
                "rejected",
            ],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "content.decision",
                "--set",
                "A dotted decision",
            ],
            &["adr", "get", "ADR-0001", "content.decision"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "govctl.title",
                "--set",
                "Compat Title",
            ],
            &["adr", "get", "ADR-0001", "govctl.title"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
