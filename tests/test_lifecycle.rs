//! Tests for lifecycle commands - RFC/ADR status and phase transitions.
//!
//! Covers: finalize, advance, accept, reject, bump, deprecate, supersede.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

macro_rules! assert_lifecycle_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        let function_name = insta::_function_name!();
        let test_name = function_name.rsplit("::").next().unwrap_or(function_name);
        let snapshot_case = test_name.strip_prefix("test_").unwrap_or(test_name);
        let snapshot_name = format!("test_lifecycle__{snapshot_case}");
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
            prepend_module_to_snapshot => false
        }, {
            insta::assert_snapshot!(snapshot_name, snapshot);
        });
    }};
}

mod lifecycle_tests;
