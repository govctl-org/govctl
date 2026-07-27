use super::*;

// ============================================================================
// Decision Gate Tests (adr set decision)
// ============================================================================

/// Setting `decision` without any alternatives must fail.
#[test]
fn test_set_decision_blocked_without_alternatives() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "decision",
            "--set",
            "We chose X.",
        ],
    ])?;

    assert_gate_error(&normalized, "decision without alternatives");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with only 1 alternative (accepted) must fail - need at least 2.
#[test]
fn test_set_decision_blocked_with_only_one_alternative() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives",
            "--add",
            "Option A",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives[0]",
            "--tick",
            "accepted",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "decision",
            "--set",
            "We chose A.",
        ],
    ])?;

    assert_gate_error(&normalized, "decision with one alternative");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with 2 alternatives but none rejected must fail.
#[test]
fn test_set_decision_blocked_without_rejected() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives",
            "--add",
            "Option A",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives",
            "--add",
            "Option B",
        ],
        // Both accepted, none rejected
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives[0]",
            "--tick",
            "accepted",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives[1]",
            "--tick",
            "accepted",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "decision",
            "--set",
            "We chose A.",
        ],
    ])?;

    assert_gate_error(&normalized, "decision without rejected alternative");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with 2 alternatives (1 accepted, 1 rejected) must succeed.
#[test]
fn test_set_decision_succeeds_with_complete_alternatives() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives",
            "--add",
            "Option A",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives",
            "--add",
            "Option B",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives[0]",
            "--tick",
            "accepted",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives[1]",
            "--tick",
            "rejected",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "decision",
            "--set",
            "We chose A.",
        ],
    ])?;

    assert_no_gate_error(&normalized, "complete decision");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with 2 alternatives but none accepted must fail.
#[test]
fn test_set_decision_blocked_without_accepted() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives",
            "--add",
            "Option A",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives",
            "--add",
            "Option B",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives[0]",
            "--tick",
            "rejected",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "alternatives[1]",
            "--tick",
            "rejected",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "decision",
            "--set",
            "We chose A.",
        ],
    ])?;

    assert_gate_error(&normalized, "decision without accepted alternative");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Storage-prefixed edit paths are outside the canonical command surface.
#[test]
fn test_storage_prefixed_decision_path_is_rejected() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "content.decision",
            "--set",
            "We chose X.",
        ],
    ])?;

    assert!(normalized.contains("error[E0803]"), "{normalized}");
    assert!(
        normalized.contains("Unknown ADR field: content.decision"),
        "{normalized}"
    );
    Ok(())
}
