// ============================================================================
// Decision Gate Tests (adr set decision)
// ============================================================================

/// Setting `decision` without any alternatives must fail.
#[test]
fn test_set_decision_blocked_without_alternatives() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &["adr", "set", "ADR-0001", "decision", "We chose X."],
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
        &["adr", "add", "ADR-0001", "alternatives", "Option A"],
        &[
            "adr",
            "tick",
            "ADR-0001",
            "alternatives",
            "--at",
            "0",
            "-s",
            "accepted",
        ],
        &["adr", "set", "ADR-0001", "decision", "We chose A."],
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
        &["adr", "add", "ADR-0001", "alternatives", "Option A"],
        &["adr", "add", "ADR-0001", "alternatives", "Option B"],
        // Both accepted, none rejected
        &[
            "adr",
            "tick",
            "ADR-0001",
            "alternatives",
            "--at",
            "0",
            "-s",
            "accepted",
        ],
        &[
            "adr",
            "tick",
            "ADR-0001",
            "alternatives",
            "--at",
            "1",
            "-s",
            "accepted",
        ],
        &["adr", "set", "ADR-0001", "decision", "We chose A."],
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
        &["adr", "add", "ADR-0001", "alternatives", "Option A"],
        &["adr", "add", "ADR-0001", "alternatives", "Option B"],
        &[
            "adr",
            "tick",
            "ADR-0001",
            "alternatives",
            "--at",
            "0",
            "-s",
            "accepted",
        ],
        &[
            "adr",
            "tick",
            "ADR-0001",
            "alternatives",
            "--at",
            "1",
            "-s",
            "rejected",
        ],
        &["adr", "set", "ADR-0001", "decision", "We chose A."],
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
        &["adr", "add", "ADR-0001", "alternatives", "Option A"],
        &["adr", "add", "ADR-0001", "alternatives", "Option B"],
        &[
            "adr",
            "tick",
            "ADR-0001",
            "alternatives",
            "--at",
            "0",
            "-s",
            "rejected",
        ],
        &[
            "adr",
            "tick",
            "ADR-0001",
            "alternatives",
            "--at",
            "1",
            "-s",
            "rejected",
        ],
        &["adr", "set", "ADR-0001", "decision", "We chose A."],
    ])?;

    assert_gate_error(&normalized, "decision without accepted alternative");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` via legacy dotted path `content.decision` must also be blocked.
#[test]
fn test_set_decision_blocked_via_legacy_dotted_path() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "set",
            "ADR-0001",
            "content.decision",
            "We chose X.",
        ],
    ])?;

    assert_gate_error(&normalized, "dotted decision path");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}
