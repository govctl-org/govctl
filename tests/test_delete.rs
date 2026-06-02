//! Delete command tests - clause and work item deletion safeguards.

mod common;

use common::{
    TestResult, command, first_work_id, init_project_with_date, normalize_output, run_commands,
    run_dynamic_commands, work_add_acceptance, work_add_dependency, work_add_field,
    work_delete_force, work_id, work_move_done, work_new, work_new_active,
    work_tick_acceptance_done,
};
use std::fs;

macro_rules! assert_delete_snapshot {
    ($value:expr) => {{
        let value = $value;
        crate::assert_current_test_snapshot!("test_delete", value);
    }};
}

mod delete_tests;
