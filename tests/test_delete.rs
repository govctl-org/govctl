//! Delete command tests - clause and work item deletion safeguards.

mod common;

use common::{
    TestResult, init_project, normalize_output, run_commands, run_dynamic_commands, today,
};
use std::fs;

macro_rules! assert_delete_snapshot {
    ($value:expr) => {{
        let value = $value;
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
        }, {
            insta::assert_snapshot!(value);
        });
    }};
}

include!("delete_tests/clause.rs");
include!("delete_tests/work.rs");
