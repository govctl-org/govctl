//! Delete command tests - clause and work item deletion safeguards.

mod common;

use common::{
    TestResult, first_work_id, init_project_with_date, normalize_output, run_commands,
    run_dynamic_commands, work_id,
};
use std::fs;

macro_rules! assert_delete_snapshot {
    ($value:expr) => {{
        let value = $value;
        let snapshot_name =
            common::current_test_snapshot_name("test_delete", insta::_function_name!());
        crate::with_test_snapshot_settings!({
            insta::assert_snapshot!(snapshot_name, value);
        });
    }};
}

mod delete_tests;
