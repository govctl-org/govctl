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
        insta::with_settings!({
            snapshot_path => Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
        }, {
            insta::assert_snapshot!(value);
        });
    }};
}

include!("display_path_tests/render.rs");
include!("display_path_tests/show.rs");
include!("display_path_tests/delete.rs");
include!("display_path_tests/edit.rs");
include!("display_path_tests/new.rs");
