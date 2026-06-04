//! Integration tests for ADR writing-order gates per [[ADR-0042]].
//!
//! Covers: decision gate (set), accept gate, --force bypass.

mod common;

const ADR_GATE_ERROR: &str = "alternatives incomplete";

fn run_gate_commands(commands: &[&[&str]]) -> Result<String, Box<dyn std::error::Error>> {
    let (dir, date) = common::init_project_with_date()?;
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
        crate::assert_current_test_snapshot!("test_adr_gates", snapshot);
    }};
}

mod adr_gate_tests;
