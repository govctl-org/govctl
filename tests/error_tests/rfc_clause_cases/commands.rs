use super::*;

#[test]
fn test_clause_commands_reject_invalid_clause_id_format() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "clause",
                "new",
                "RFC-0001:C-ONE:EXTRA",
                "Bad Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["clause", "show", "RFC-0001:C-ONE:EXTRA"],
            &["clause", "delete", "RFC-0001:C-ONE:EXTRA", "-f"],
            &["clause", "deprecate", "RFC-0001:C-ONE:EXTRA", "-f"],
        ],
    )?;

    assert_eq!(
        output.matches("exit: 1").count(),
        4,
        "all malformed clause commands should fail: {output}"
    );
    assert!(
        output.contains("Invalid clause ID format. Expected RFC-NNNN:C-NAME"),
        "new/delete invalid-ID diagnostics should stay stable: {output}"
    );
    assert!(
        output
            .contains("Invalid clause ID format: RFC-0001:C-ONE:EXTRA (expected RFC-NNNN:C-NAME)"),
        "show invalid-ID diagnostic should stay stable: {output}"
    );
    assert!(
        output.contains("Clause not found: RFC-0001:C-ONE:EXTRA"),
        "lifecycle malformed-ID lookup should stay in the not-found path: {output}"
    );
    Ok(())
}
