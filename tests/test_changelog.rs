//! Changelog and release workflow tests.

mod common;

use common::{normalize_output, run_commands, run_dynamic_commands, today};
use tempfile::TempDir;

macro_rules! assert_changelog_snapshot {
    ($name:literal, $value:expr) => {{
        let value = $value;
        let snapshot_name = concat!("test_changelog__", $name);
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
            prepend_module_to_snapshot => false
        }, {
            insta::assert_snapshot!(snapshot_name, value);
        });
    }};
}

mod changelog_tests;
