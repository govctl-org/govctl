//! Tests for edit commands - modifying artifact fields.

mod common;

use common::{init_project, normalize_output, run_commands, today};

macro_rules! assert_edit_snapshot {
    ($value:expr) => {{
        let value = $value;
        insta::with_settings!({ snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots") }, {
            insta::assert_snapshot!(value);
        });
    }};
}

include!("edit_tests/rfc.rs");
include!("edit_tests/clause.rs");
include!("edit_tests/adr.rs");
include!("edit_tests/work.rs");
include!("edit_tests/paths.rs");
