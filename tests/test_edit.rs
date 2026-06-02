//! Tests for edit commands - modifying artifact fields.

mod common;

use common::{init_project, init_project_with_date, normalize_output, run_commands};

macro_rules! assert_edit_snapshot {
    ($value:expr) => {{
        let value = $value;
        crate::assert_current_test_snapshot!("test_edit", value);
    }};
}

mod edit_tests;
