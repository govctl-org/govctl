//! Common helpers shared across integration test binaries.

#![allow(dead_code)] // Functions used across different test binaries

mod commands;
mod fixtures;
pub mod loop_helpers;
mod snapshots;

// Each integration test binary imports a different subset of this facade.
#[allow(unused_imports)]
pub use commands::{
    command, format_command_output, run_commands, run_dynamic_commands, run_normalized_commands,
    work_add_acceptance, work_add_dependency, work_add_field, work_delete_force, work_get_field,
    work_list_all, work_move_done, work_new, work_new_active, work_remove_acceptance,
    work_remove_dependency, work_remove_field, work_set_field, work_show, work_tick_acceptance,
    work_tick_acceptance_done,
};
#[allow(unused_imports)]
pub use fixtures::{
    append_verification_config, first_work_id, init_project, init_project_at, init_project_v1,
    init_project_with_date, temp_dir_with_date, today, work_id, write_canonical_guarded_work_item,
    write_guard, write_guard_with_timeout, write_guarded_work_item,
};
#[allow(unused_imports)]
pub use snapshots::{
    current_test_snapshot_name, named_snapshot_name, normalize_output, snapshot_path,
};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

#[macro_export]
macro_rules! with_test_snapshot_settings {
    ($body:block) => {{
        insta::with_settings!({
            snapshot_path => $crate::common::snapshot_path(),
            prepend_module_to_snapshot => false
        }, $body);
    }};
}

#[macro_export]
macro_rules! assert_current_test_snapshot {
    ($prefix:expr, $value:expr $(,)?) => {{
        let snapshot_name =
            $crate::common::current_test_snapshot_name($prefix, insta::_function_name!());
        $crate::with_test_snapshot_settings!({
            insta::assert_snapshot!(snapshot_name, $value);
        });
    }};
}

#[macro_export]
macro_rules! assert_normalized_command_snapshot {
    ($prefix:expr, $dir:expr, $date:expr, $commands:expr $(,)?) => {{
        let value = $crate::common::run_normalized_commands($dir, $date, $commands)?;
        $crate::assert_current_test_snapshot!($prefix, value);
    }};
}
