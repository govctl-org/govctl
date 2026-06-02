//! Tests for lifecycle commands - RFC/ADR status and phase transitions.
//!
//! Covers: finalize, advance, accept, reject, bump, deprecate, supersede.

mod common;

use common::{init_project, init_project_with_date, normalize_output, run_commands};
use std::fs;

macro_rules! assert_lifecycle_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        let snapshot_name =
            common::current_test_snapshot_name("test_lifecycle", insta::_function_name!());
        crate::with_test_snapshot_settings!({
            insta::assert_snapshot!(snapshot_name, snapshot);
        });
    }};
}

mod lifecycle_tests;
