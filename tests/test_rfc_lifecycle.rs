//! RFC lifecycle and amendment tracking tests.

mod common;

use common::{init_project, normalize_output, run_commands, today};

/// Test: RFC amendment tracking with signature-based detection
#[test]
fn test_rfc_amendment_tracking() {
    let temp_dir = init_project();
    let date = today();

    // Create RFC and clause
    let setup = run_commands(
        temp_dir.path(),
        &[
            &["new", "rfc", "Amendment Test RFC"],
            &[
                "new",
                "clause",
                "RFC-0001:C-AMEND-TEST",
                "Amendment Test Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &[
                "edit",
                "RFC-0001:C-AMEND-TEST",
                "--text",
                "Original text for amendment test.",
            ],
        ],
    );

    // Finalize and bump to set baseline signature
    let baseline = run_commands(
        temp_dir.path(),
        &[
            &["finalize", "RFC-0001", "normative"],
            &[
                "bump",
                "RFC-0001",
                "--minor",
                "--summary",
                "Establish baseline with signature",
            ],
            &["list", "rfc"],
        ],
    );

    // Edit clause to create amendment
    let edit = run_commands(
        temp_dir.path(),
        &[&[
            "edit",
            "RFC-0001:C-AMEND-TEST",
            "--text",
            "AMENDED text - content changed.",
        ]],
    );

    // List should show asterisk for amended RFC
    let amended = run_commands(temp_dir.path(), &[&["list", "rfc"]]);

    // Bump version to release amendment
    let released = run_commands(
        temp_dir.path(),
        &[
            &[
                "bump",
                "RFC-0001",
                "--patch",
                "--summary",
                "Release amendment",
            ],
            &["list", "rfc"],
        ],
    );

    let combined = format!("{}{}{}{}{}", setup, baseline, edit, amended, released);
    insta::assert_snapshot!(normalize_output(&combined, temp_dir.path(), &date));
}
