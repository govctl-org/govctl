//! Tests for edit commands - modifying artifact fields.

mod common;

use common::{init_project, init_project_with_date, normalize_output, run_commands};

macro_rules! assert_edit_snapshot {
    ($value:expr) => {{
        let value = $value;
        let snapshot_name =
            common::current_test_snapshot_name("test_edit", insta::_function_name!());
        crate::with_test_snapshot_settings!({
            insta::assert_snapshot!(snapshot_name, value);
        });
    }};
}

mod edit_tests;
