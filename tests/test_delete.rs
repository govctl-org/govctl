//! Delete command tests - clause and work item deletion safeguards.

mod common;

use common::{
    TestResult, init_project, normalize_output, run_commands, run_dynamic_commands, today,
};
use std::fs;

macro_rules! assert_delete_snapshot {
    ($value:expr) => {{
        let value = $value;
        let function_name = insta::_function_name!();
        let test_name = function_name.rsplit("::").next().unwrap_or(function_name);
        let snapshot_case = test_name.strip_prefix("test_").unwrap_or(test_name);
        let snapshot_name = format!("test_delete__{snapshot_case}");
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
            prepend_module_to_snapshot => false
        }, {
            insta::assert_snapshot!(snapshot_name, value);
        });
    }};
}

mod delete_tests;
