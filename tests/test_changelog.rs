//! Changelog and release workflow tests.

mod common;

use common::{normalize_output, run_commands, run_dynamic_commands, today};
use tempfile::TempDir;

macro_rules! assert_changelog_snapshot {
    ($name:literal, $value:expr) => {{
        let value = $value;
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
        }, {
            insta::assert_snapshot!($name, value);
        });
    }};
}

include!("changelog_tests/release_workflow.rs");
include!("changelog_tests/preservation.rs");
