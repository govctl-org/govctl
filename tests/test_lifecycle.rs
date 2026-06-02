//! Tests for lifecycle commands - RFC/ADR status and phase transitions.
//!
//! Covers: finalize, advance, accept, reject, bump, deprecate, supersede.

mod common;

use common::{init_project, init_project_with_date, normalize_output, run_commands};
use std::fs;

macro_rules! assert_lifecycle_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        crate::assert_current_test_snapshot!("test_lifecycle", snapshot);
    }};
}

mod lifecycle_tests;
