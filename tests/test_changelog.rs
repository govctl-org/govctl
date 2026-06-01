//! Changelog and release workflow tests.

mod common;

use common::{normalize_output, run_commands, run_dynamic_commands, today};
use tempfile::TempDir;

macro_rules! assert_changelog_snapshot {
    ($name:literal, $value:expr) => {{
        let value = $value;
        let snapshot_name = common::named_snapshot_name("test_changelog", $name);
        crate::with_test_snapshot_settings!({
            insta::assert_snapshot!(snapshot_name, value);
        });
    }};
}

mod changelog_tests;
