use super::*;

// =========================================================================
// RFC Status Transition Tests
// =========================================================================

#[test]
fn test_rfc_status_draft_to_normative() {
    assert!(is_valid_status_transition(
        RfcStatus::Draft,
        RfcStatus::Normative
    ));
}

#[test]
fn test_rfc_status_normative_to_deprecated() {
    assert!(is_valid_status_transition(
        RfcStatus::Normative,
        RfcStatus::Deprecated
    ));
}

#[test]
fn test_rfc_status_invalid_draft_to_deprecated() {
    assert!(!is_valid_status_transition(
        RfcStatus::Draft,
        RfcStatus::Deprecated
    ));
}

#[test]
fn test_rfc_status_invalid_normative_to_draft() {
    assert!(!is_valid_status_transition(
        RfcStatus::Normative,
        RfcStatus::Draft
    ));
}

#[test]
fn test_rfc_status_invalid_deprecated_to_normative() {
    assert!(!is_valid_status_transition(
        RfcStatus::Deprecated,
        RfcStatus::Normative
    ));
}

#[test]
fn test_rfc_status_same_state() {
    assert!(!is_valid_status_transition(
        RfcStatus::Draft,
        RfcStatus::Draft
    ));
    assert!(!is_valid_status_transition(
        RfcStatus::Normative,
        RfcStatus::Normative
    ));
}

// =========================================================================
// RFC Phase Transition Tests
// =========================================================================

#[test]
fn test_rfc_phase_spec_to_impl() {
    assert!(is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Impl));
}

#[test]
fn test_rfc_phase_impl_to_test() {
    assert!(is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Test));
}

#[test]
fn test_rfc_phase_test_to_stable() {
    assert!(is_valid_phase_transition(RfcPhase::Test, RfcPhase::Stable));
}

#[test]
fn test_rfc_phase_invalid_skip() {
    // Cannot skip phases.
    assert!(!is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Test));
    assert!(!is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Stable));
    assert!(!is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Stable));
}

#[test]
fn test_rfc_phase_invalid_backward() {
    assert!(!is_valid_phase_transition(RfcPhase::Stable, RfcPhase::Test));
    assert!(!is_valid_phase_transition(RfcPhase::Test, RfcPhase::Impl));
    assert!(!is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Spec));
}

// =========================================================================
// ADR Status Transition Tests
// =========================================================================

#[test]
fn test_adr_status_proposed_to_accepted() {
    assert!(is_valid_adr_transition(
        AdrStatus::Proposed,
        AdrStatus::Accepted
    ));
}

#[test]
fn test_adr_status_accepted_to_superseded() {
    assert!(is_valid_adr_transition(
        AdrStatus::Accepted,
        AdrStatus::Superseded
    ));
}

#[test]
fn test_adr_status_proposed_to_rejected() {
    assert!(is_valid_adr_transition(
        AdrStatus::Proposed,
        AdrStatus::Rejected
    ));
}

#[test]
fn test_adr_status_invalid_proposed_to_superseded() {
    assert!(!is_valid_adr_transition(
        AdrStatus::Proposed,
        AdrStatus::Superseded
    ));
}

#[test]
fn test_adr_status_invalid_rejected_transitions() {
    // Rejected is terminal.
    assert!(!is_valid_adr_transition(
        AdrStatus::Rejected,
        AdrStatus::Accepted
    ));
    assert!(!is_valid_adr_transition(
        AdrStatus::Rejected,
        AdrStatus::Proposed
    ));
}

#[test]
fn test_adr_status_invalid_backward() {
    assert!(!is_valid_adr_transition(
        AdrStatus::Accepted,
        AdrStatus::Proposed
    ));
    assert!(!is_valid_adr_transition(
        AdrStatus::Superseded,
        AdrStatus::Accepted
    ));
}

// =========================================================================
// Work Item Status Transition Tests
// =========================================================================

#[test]
fn test_work_status_queue_to_active() {
    assert!(is_valid_work_transition(
        WorkItemStatus::Queue,
        WorkItemStatus::Active
    ));
}

#[test]
fn test_work_status_active_to_done() {
    assert!(is_valid_work_transition(
        WorkItemStatus::Active,
        WorkItemStatus::Done
    ));
}

#[test]
fn test_work_status_queue_to_cancelled() {
    assert!(is_valid_work_transition(
        WorkItemStatus::Queue,
        WorkItemStatus::Cancelled
    ));
}

#[test]
fn test_work_status_active_to_cancelled() {
    assert!(is_valid_work_transition(
        WorkItemStatus::Active,
        WorkItemStatus::Cancelled
    ));
}

#[test]
fn test_work_status_invalid_queue_to_done() {
    // Cannot skip active.
    assert!(!is_valid_work_transition(
        WorkItemStatus::Queue,
        WorkItemStatus::Done
    ));
}

#[test]
fn test_work_status_invalid_done_transitions() {
    // Done is terminal.
    assert!(!is_valid_work_transition(
        WorkItemStatus::Done,
        WorkItemStatus::Active
    ));
    assert!(!is_valid_work_transition(
        WorkItemStatus::Done,
        WorkItemStatus::Queue
    ));
}

#[test]
fn test_work_status_invalid_cancelled_transitions() {
    // Cancelled is terminal.
    assert!(!is_valid_work_transition(
        WorkItemStatus::Cancelled,
        WorkItemStatus::Active
    ));
    assert!(!is_valid_work_transition(
        WorkItemStatus::Cancelled,
        WorkItemStatus::Queue
    ));
}
