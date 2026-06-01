//! Error case tests - validation of invalid states.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

macro_rules! assert_error_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        let snapshot_name =
            common::current_test_snapshot_name("test_errors", insta::_function_name!());
        crate::with_test_snapshot_settings!({
            insta::assert_snapshot!(snapshot_name, snapshot);
        });
    }};
}

mod error_tests;
