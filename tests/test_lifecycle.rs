//! Tests for lifecycle commands - RFC/ADR status and phase transitions.
//!
//! Covers: finalize, advance, accept, reject, bump, deprecate, supersede.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

macro_rules! assert_lifecycle_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
        }, {
            insta::assert_snapshot!(snapshot);
        });
    }};
}

include!("lifecycle_tests/rfc.rs");
include!("lifecycle_tests/adr.rs");
include!("lifecycle_tests/clause.rs");
