use super::*;

#[test]
fn test_path_backward_compat() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Compat Test"],
            // Implements [[ADR-0042]]: must have alternatives before setting decision
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
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
