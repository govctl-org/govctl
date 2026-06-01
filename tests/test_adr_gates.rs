//! Integration tests for ADR writing-order gates per [[ADR-0042]].
//!
//! Covers: decision gate (set), accept gate, --force bypass.

mod common;

const ADR_GATE_ERROR: &str = "alternatives incomplete";

fn run_gate_commands(commands: &[&[&str]]) -> Result<String, Box<dyn std::error::Error>> {
    let dir = common::init_project()?;
    let date = common::today();
    let output = common::run_commands(dir.path(), commands)?;
    Ok(common::normalize_output(&output, dir.path(), &date)?)
}

fn assert_gate_error(normalized: &str, context: &str) {
    assert!(
        normalized.contains(ADR_GATE_ERROR),
        "expected gate error for {context}, got: {normalized}"
    );
}

fn assert_no_gate_error(normalized: &str, context: &str) {
    assert!(
        !normalized.contains(ADR_GATE_ERROR),
        "unexpected gate error for {context}: {normalized}"
    );
}

macro_rules! assert_adr_gate_snapshot {
    ($value:expr) => {{
        let snapshot = $value;
        let function_name = insta::_function_name!();
        let test_name = function_name.rsplit("::").next().unwrap_or(function_name);
        let snapshot_case = test_name.strip_prefix("test_").unwrap_or(test_name);
        let snapshot_name = format!("test_adr_gates__{snapshot_case}");
        insta::with_settings!({
            snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
            prepend_module_to_snapshot => false
        }, {
            insta::assert_snapshot!(snapshot_name, snapshot);
        });
    }};
}

mod adr_gate_tests;
