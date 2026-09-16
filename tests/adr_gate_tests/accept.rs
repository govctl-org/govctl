use super::*;

// ============================================================================
// Accept Gate Tests (adr accept)
// ============================================================================

/// Accepting an ADR with no alternatives at all must fail.
#[test]
fn test_accept_blocked_without_alternatives() -> common::TestResult {
    let normalized =
        run_gate_commands(&[&["adr", "new", "Test ADR"], &["adr", "accept", "ADR-0001"]])?;

    assert_gate_error(&normalized, "accept without alternatives");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Accepting an ADR with 1 accepted alternative must succeed per [[ADR-0063]].
#[test]
fn test_accept_succeeds_with_one_accepted_alternative() -> common::TestResult {
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
            "We chose Option A.",
        ],
        &["adr", "accept", "ADR-0001"],
        &["adr", "list"],
    ])?;

    assert_no_gate_error(&normalized, "accept with one accepted alternative");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Accepting an ADR with 2 alternatives but none rejected must fail per [[ADR-0063]].
#[test]
fn test_accept_blocked_without_rejected_with_two_alternatives() -> common::TestResult {
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
        &["adr", "accept", "ADR-0001"],
    ])?;

    assert_gate_error(
        &normalized,
        "accept with two alternatives and none rejected",
    );
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Accepting an ADR with 2 alternatives but none accepted must fail.
#[test]
fn test_accept_blocked_without_accepted() -> common::TestResult {
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
        &["adr", "accept", "ADR-0001"],
    ])?;

    assert_gate_error(&normalized, "accept without accepted alternative");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// `adr accept --force` bypasses alternatives-completeness gates.
#[test]
fn test_accept_force_bypasses_gates() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        // No alternatives, no decision - force should bypass all gates
        &["adr", "accept", "ADR-0001", "--force"],
        &["adr", "list"],
    ])?;

    assert_no_gate_error(&normalized, "forced accept");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// A fully complete ADR (2 alts, 1 accepted, 1 rejected) can be accepted without --force.
#[test]
fn test_accept_succeeds_with_complete_adr() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "context",
            "--set",
            "We need to pick a storage layer.",
        ],
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
            "We chose Option A.",
        ],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "consequences",
            "--set",
            "Faster reads, more memory.",
        ],
        &["adr", "accept", "ADR-0001"],
        &["adr", "list"],
    ])?;

    assert_no_gate_error(&normalized, "complete accept");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

#[test]
fn test_accept_force_does_not_bypass_projection_ownership() -> common::TestResult {
    let normalized = run_gate_commands(&[
        &["adr", "new", "Test ADR"],
        &[
            "adr",
            "edit",
            "ADR-0001",
            "context",
            "--set",
            "### Options Considered",
        ],
        &["adr", "accept", "ADR-0001", "--force"],
        &["adr", "get", "ADR-0001", "status"],
    ])?;

    assert!(normalized.contains("error[E0307]"), "{normalized}");
    assert!(normalized.contains("content.context"), "{normalized}");
    assert!(
        normalized.contains("heading 'Options Considered'"),
        "{normalized}"
    );
    assert!(
        normalized.contains("$ govctl adr get ADR-0001 status\nproposed"),
        "{normalized}"
    );
    Ok(())
}
