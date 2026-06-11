//! Error case tests - validation of invalid states.

mod common;

use common::{
    init_project, init_project_with_date, normalize_output, run_commands, write_minimal_rfc,
};
use std::fs;

macro_rules! assert_error_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        crate::assert_current_test_snapshot!("test_errors", snapshot);
    }};
}

mod error_tests;
