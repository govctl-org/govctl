//! Tests for source code reference scanning.
//!
//! Tests the scan_source_refs function which scans source files for
//! references to governance artifacts.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

/// Helper to enable source scanning in a project
fn enable_source_scan(dir: &std::path::Path) {
    let config_path = dir.join("gov/config.toml");
    let config = fs::read_to_string(&config_path).unwrap();
    // Add source_scan section if not present
    if !config.contains("[source_scan]") {
        let updated = format!(
            "{}\n[source_scan]\nenabled = true\ninclude = [\"src/**/*.rs\"]\nexclude = []\n",
            config
        );
        fs::write(&config_path, updated).unwrap();
    }
}

#[test]
fn test_scan_no_references() {
    // project with no source files should scan successfully
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_valid_rfc_reference() {
    // source file with valid RFC reference should pass
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create an RFC
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    );

    // Create a source file with a reference to the RFC
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_valid_clause_reference() {
    // source file with valid clause reference should pass
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create an RFC with a clause
    run_commands(
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
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    );

    // Create a source file with a reference to the clause
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Implements [[RFC-0001:C-TEST]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_unknown_rfc_reference() {
    // source file with unknown RFC reference should error
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create a source file with a reference to non-existent RFC
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Implements [[RFC-9999]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_unknown_clause_reference() {
    // source file with unknown clause reference should error
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create an RFC but no clause
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    );

    // Create a source file with a reference to non-existent clause
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Implements [[RFC-0001:C-NONEXISTENT]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_deprecated_rfc_reference() {
    // source file with deprecated RFC reference should warn
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create and deprecate an RFC
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001"],
        ],
    );

    // Create a source file with a reference to deprecated RFC
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_valid_adr_reference() {
    // source file with valid ADR reference should pass
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create an ADR
    run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "accept", "ADR-0001"],
        ],
    );

    // Create a source file with a reference to the ADR
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Follows [[ADR-0001]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_valid_work_item_reference() {
    // source file with valid work item reference should pass
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create a work item
    run_commands(temp_dir.path(), &[&["work", "new", "Test task"]]);

    // Get the work item ID from the output
    let wi_output = run_commands(temp_dir.path(), &[&["work", "list", "all"]]);
    let wi_id = regex::Regex::new(r"WI-\d{4}-\d{2}-\d{2}-\d{3}")
        .unwrap()
        .find(&wi_output)
        .map(|m| m.as_str().to_string())
        .expect("No work item ID found");

    // Create a source file with a reference to the work item
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        format!("// Implements [[{}]]\nfn main() {{}}\n", wi_id),
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_multiple_references_in_file() {
    // source file with multiple references should check all
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create RFCs
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "RFC One"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "new", "RFC Two"],
            &["rfc", "finalize", "RFC-0002", "normative"],
        ],
    );

    // Create a source file with multiple references
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Implements [[RFC-0001]] and [[RFC-0002]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_scan_mixed_valid_invalid_references() {
    // source file with mixed valid/invalid references should report errors
    let temp_dir = init_project();
    enable_source_scan(temp_dir.path());
    let date = today();

    // Create one RFC
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Valid RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    );

    // Create a source file with valid and invalid references
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "// Implements [[RFC-0001]] and [[RFC-9999]]\nfn main() {}\n",
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
