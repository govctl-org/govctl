//! Integration tests for ADR writing-order gates per [[ADR-0042]].
//!
//! Covers: decision gate (set), accept gate, --force bypass.

mod common;

// ============================================================================
// Decision Gate Tests (adr set decision)
// ============================================================================

/// Setting `decision` without any alternatives must fail.
#[test]
fn test_set_decision_blocked_without_alternatives() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
            &["adr", "new", "Test ADR"],
            &["adr", "set", "ADR-0001", "decision", "We chose X."],
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with only 1 alternative (accepted) must fail — need at least 2.
#[test]
fn test_set_decision_blocked_with_only_one_alternative() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
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
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with 2 alternatives but none rejected must fail.
#[test]
fn test_set_decision_blocked_without_rejected() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
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
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with 2 alternatives (1 accepted, 1 rejected) must succeed.
#[test]
fn test_set_decision_succeeds_with_complete_alternatives() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
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
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        !normalized.contains("alternatives incomplete"),
        "unexpected gate error: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` with 2 alternatives but none accepted must fail.
#[test]
fn test_set_decision_blocked_without_accepted() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
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
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// Setting `decision` via legacy dotted path `content.decision` must also be blocked.
#[test]
fn test_set_decision_blocked_via_legacy_dotted_path() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
            &["adr", "new", "Test ADR"],
            &["adr", "set", "ADR-0001", "content.decision", "We chose X."],
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error for dotted path, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

// ============================================================================
// Accept Gate Tests (adr accept)
// ============================================================================

/// Accepting an ADR with no alternatives at all must fail.
#[test]
fn test_accept_blocked_without_alternatives() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[&["adr", "new", "Test ADR"], &["adr", "accept", "ADR-0001"]],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// Accepting an ADR with only 1 alternative must fail (need at least 2).
#[test]
fn test_accept_blocked_with_only_one_alternative() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
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
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// Accepting an ADR with 2 alternatives but none accepted must fail.
#[test]
fn test_accept_blocked_without_accepted() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
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
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        normalized.contains("alternatives incomplete"),
        "expected gate error, got: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// `adr accept --force` bypasses alternatives-completeness gates.
#[test]
fn test_accept_force_bypasses_gates() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
            &["adr", "new", "Test ADR"],
            // No alternatives, no decision — force should bypass all gates
            &["adr", "accept", "ADR-0001", "--force"],
            &["adr", "list"],
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        !normalized.contains("alternatives incomplete"),
        "unexpected gate error with --force: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}

/// A fully complete ADR (2 alts, 1 accepted, 1 rejected) can be accepted without --force.
#[test]
fn test_accept_succeeds_with_complete_adr() -> common::TestResult {
    let dir = common::init_project()?;
    let date = common::today();

    let output = common::run_commands(
        dir.path(),
        &[
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
        ],
    )?;
    let normalized = common::normalize_output(&output, dir.path(), &date)?;
    assert!(
        !normalized.contains("alternatives incomplete"),
        "unexpected gate error: {normalized}"
    );
    insta::assert_snapshot!(normalized);
    Ok(())
}
