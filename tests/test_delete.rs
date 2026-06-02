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
        crate::assert_current_test_snapshot!("test_delete", value);
    }};
}

mod delete_tests;
