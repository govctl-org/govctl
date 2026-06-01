//! Tests for edit commands - modifying artifact fields.

mod common;

use common::{init_project, normalize_output, run_commands, today};

macro_rules! assert_edit_snapshot {
    ($value:expr) => {{
        let value = $value;
        let function_name = insta::_function_name!();
        let test_name = function_name.rsplit("::").next().unwrap_or(function_name);
        let snapshot_case = test_name.strip_prefix("test_").unwrap_or(test_name);
        let snapshot_name = format!("test_edit__{snapshot_case}");
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
            prepend_module_to_snapshot => false
        }, {
            insta::assert_snapshot!(snapshot_name, value);
        });
    }};
}

mod edit_tests;
