use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};

/// Check if RFC status transition is valid.
pub fn is_valid_status_transition(from: RfcStatus, to: RfcStatus) -> bool {
    matches!(
        (from, to),
        (RfcStatus::Draft, RfcStatus::Normative) | (RfcStatus::Normative, RfcStatus::Deprecated)
    )
}

/// Check if RFC phase transition is valid.
pub fn is_valid_phase_transition(from: RfcPhase, to: RfcPhase) -> bool {
    matches!(
        (from, to),
        (RfcPhase::Spec, RfcPhase::Impl)
            | (RfcPhase::Impl, RfcPhase::Test)
            | (RfcPhase::Test, RfcPhase::Stable)
    )
}

/// Check if ADR status transition is valid.
///
/// ADR lifecycle:
/// - proposed -> accepted -> superseded
/// - proposed -> rejected
pub fn is_valid_adr_transition(from: AdrStatus, to: AdrStatus) -> bool {
    matches!(
        (from, to),
        (AdrStatus::Proposed, AdrStatus::Accepted)
            | (AdrStatus::Proposed, AdrStatus::Rejected)
            | (AdrStatus::Accepted, AdrStatus::Superseded)
    )
}

/// Check if Work Item status transition is valid.
pub fn is_valid_work_transition(from: WorkItemStatus, to: WorkItemStatus) -> bool {
    matches!(
        (from, to),
        (WorkItemStatus::Queue, WorkItemStatus::Active)
            | (WorkItemStatus::Active, WorkItemStatus::Done)
            | (WorkItemStatus::Done, WorkItemStatus::Active)
            | (WorkItemStatus::Queue, WorkItemStatus::Cancelled)
            | (WorkItemStatus::Active, WorkItemStatus::Cancelled)
    )
}

#[cfg(test)]
mod tests;
