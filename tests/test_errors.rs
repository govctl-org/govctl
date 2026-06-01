//! Error case tests - validation of invalid states.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

macro_rules! assert_error_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
        }, {
            insta::assert_snapshot!(snapshot);
        });
    }};
}

include!("error_tests/schema.rs");
include!("error_tests/rfc_clause.rs");
include!("error_tests/work.rs");
