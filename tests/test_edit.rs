//! Tests for edit commands - modifying artifact fields.

mod common;

use common::{
    command, first_work_id, init_project, init_project_with_date, normalize_output, run_commands,
    work_add_acceptance, work_add_dependency, work_add_field, work_get_field, work_id,
    work_list_all, work_new, work_remove_acceptance, work_remove_dependency, work_set_field,
    work_show, work_tick_acceptance,
};

macro_rules! assert_edit_snapshot {
    ($value:expr) => {{
        let value = $value;
        crate::assert_current_test_snapshot!("test_edit", value);
    }};
}

mod edit_tests;
