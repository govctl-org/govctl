//! RFC lifecycle and amendment tracking tests.

mod common;

use common::{init_project_with_date, normalize_output, run_commands};

#[test]
fn test_rfc_amendment_tracking() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Create RFC and clause
    let setup = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Amendment Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-AMEND-TEST",
                "Amendment Test Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "clause",
                "edit",
                "RFC-0001:C-AMEND-TEST",
                "--text",
                "Original text for amendment test.",
            ],
        ],
    )?;

    // Enter implementation to seal the initial version baseline.
    let baseline = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &["rfc", "list"],
        ],
    )?;

    // Edit clause to create amendment
    let edit = run_commands(
        temp_dir.path(),
        &[&[
            "clause",
            "edit",
            "RFC-0001:C-AMEND-TEST",
            "--text",
            "AMENDED text - content changed.",
        ]],
    )?;

    // List should show asterisk for amended RFC
    let amended = run_commands(temp_dir.path(), &[&["rfc", "list"]])?;

    // Bump version to release amendment
    let released = run_commands(
        temp_dir.path(),
        &[
            &[
                "rfc",
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Release amendment",
            ],
            &["rfc", "list"],
        ],
    )?;

    let combined = format!("{}{}{}{}{}", setup, baseline, edit, amended, released);
    let normalized = normalize_output(&combined, temp_dir.path(), &date)?;
    crate::assert_current_test_snapshot!("test_rfc_lifecycle", normalized);
    Ok(())
}
