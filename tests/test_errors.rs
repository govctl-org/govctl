//! Error case tests - validation of invalid states.

mod common;

use common::{
    git_ceiling_above, git_init_isolated, init_project, init_project_with_date, normalize_output,
    run_commands, run_commands_with_env, write_minimal_rfc,
};
use std::fs;

macro_rules! assert_error_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        crate::assert_current_test_snapshot!("test_errors", snapshot);
    }};
}

mod error_tests;
