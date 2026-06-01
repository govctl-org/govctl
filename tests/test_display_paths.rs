//! Display path tests - verify relative paths in output.
//!
//! These tests ensure that paths shown to users are relative to the project root,
//! not absolute paths that would vary across machines.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;
use std::path::Path;

fn assert_show_missing_scope(output: &str, temp_dir: &Path, error: &str, scope: &str) {
    assert!(output.contains(error), "{output}");
    assert!(output.contains(scope), "{output}");
    assert!(
        !output.contains(&temp_dir.display().to_string()),
        "show output should not contain absolute temp path: {output}"
    );
    if let Ok(canonical) = temp_dir.canonicalize() {
        assert!(
            !output.contains(&canonical.display().to_string()),
            "show output should not contain canonical temp path: {output}"
        );
    }
}

macro_rules! assert_display_path_snapshot {
    ($value:expr) => {{
        let value = $value;
        let function_name = insta::_function_name!();
        let test_name = function_name.rsplit("::").next().unwrap_or(function_name);
        let snapshot_case = test_name.strip_prefix("test_").unwrap_or(test_name);
        let snapshot_name = format!("test_display_paths__{snapshot_case}");
        insta::with_settings!({
            snapshot_path => Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
            prepend_module_to_snapshot => false
        }, {
            insta::assert_snapshot!(snapshot_name, value);
        });
    }};
}

mod display_path_tests;
