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

/// Accepting an ADR with only 1 alternative must fail (need at least 2).
#[test]
fn test_accept_blocked_with_only_one_alternative() -> common::TestResult {
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
        &["adr", "accept", "ADR-0001"],
    ])?;

    assert_gate_error(&normalized, "accept with one alternative");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}

/// Accepting an ADR with 2 alternatives but none accepted must fail.
#[test]
fn test_accept_blocked_without_accepted() -> common::TestResult {
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
            "set",
            "ADR-0001",
            "context",
            "We need to pick a storage layer.",
        ],
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
        &["adr", "set", "ADR-0001", "decision", "We chose Option A."],
        &[
            "adr",
            "set",
            "ADR-0001",
            "consequences",
            "Faster reads, more memory.",
        ],
        &["adr", "accept", "ADR-0001"],
        &["adr", "list"],
    ])?;

    assert_no_gate_error(&normalized, "complete accept");
    assert_adr_gate_snapshot!(normalized);
    Ok(())
}
