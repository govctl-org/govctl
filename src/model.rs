//! Data models for all governed artifacts.
//!
//! Implements data structures per [[RFC-0000]] governance framework:
//! - RFCs with clauses ([[RFC-0000:C-RFC-DEF]])
//! - ADRs ([[RFC-0000:C-ADR-DEF]])
//! - Work Items ([[RFC-0000:C-WORK-DEF]])
//!
//! Lifecycle state machines per [[RFC-0001]].

mod adr;
mod guard;
mod index;
mod release;
mod rfc;
mod work;

pub use adr::{AdrContent, AdrMeta, AdrSpec, AdrStatus, Alternative, AlternativeStatus};
pub use guard::{GuardCheck, GuardMeta, GuardSpec};
pub use index::{AdrEntry, ClauseEntry, GuardEntry, ProjectIndex, RfcIndex, WorkItemEntry};
#[allow(unused_imports)]
pub use release::ReleasesMeta;
pub use release::{Release, ReleasesFile};
pub use rfc::{
    ChangelogEntry, ClauseKind, ClauseSpec, ClauseStatus, ClauseWire, RfcPhase, RfcSpec, RfcStatus,
    RfcWire, SectionSpec,
};
pub use work::{
    ChangelogCategory, ChecklistItem, ChecklistStatus, WorkItemContent, WorkItemMeta, WorkItemSpec,
    WorkItemStatus, WorkItemVerification,
};
#[allow(unused_imports)]
pub use work::{GuardWaiver, JournalEntry};

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ChecklistItem Tests
    // =========================================================================

    #[test]
    fn test_checklist_item_new() {
        let item = ChecklistItem::new("Test criterion");
        assert_eq!(item.text, "Test criterion");
        assert_eq!(item.status, ChecklistStatus::Pending);
    }

    #[test]
    fn test_checklist_item_new_from_string() {
        let item = ChecklistItem::new(String::from("From String"));
        assert_eq!(item.text, "From String");
        assert_eq!(item.status, ChecklistStatus::Pending);
    }

    // =========================================================================
    // Alternative Tests
    // =========================================================================

    #[test]
    fn test_alternative_new() {
        let alt = Alternative::new("Use Redis for caching");
        assert_eq!(alt.text, "Use Redis for caching");
        assert_eq!(alt.status, AlternativeStatus::Considered);
    }

    #[test]
    fn test_alternative_new_from_string() {
        let alt = Alternative::new(String::from("Use PostgreSQL"));
        assert_eq!(alt.text, "Use PostgreSQL");
        assert_eq!(alt.status, AlternativeStatus::Considered);
    }

    // =========================================================================
    // Enum Default Tests
    // =========================================================================

    #[test]
    fn test_checklist_status_default() {
        assert_eq!(ChecklistStatus::default(), ChecklistStatus::Pending);
    }

    #[test]
    fn test_alternative_status_default() {
        assert_eq!(AlternativeStatus::default(), AlternativeStatus::Considered);
    }

    #[test]
    fn test_clause_status_default() {
        assert_eq!(ClauseStatus::default(), ClauseStatus::Active);
    }

    // =========================================================================
    // AsRef Tests (strum)
    // =========================================================================

    #[test]
    fn test_rfc_status_as_ref() {
        assert_eq!(RfcStatus::Draft.as_ref(), "draft");
        assert_eq!(RfcStatus::Normative.as_ref(), "normative");
        assert_eq!(RfcStatus::Deprecated.as_ref(), "deprecated");
    }

    #[test]
    fn test_rfc_phase_as_ref() {
        assert_eq!(RfcPhase::Spec.as_ref(), "spec");
        assert_eq!(RfcPhase::Impl.as_ref(), "impl");
        assert_eq!(RfcPhase::Test.as_ref(), "test");
        assert_eq!(RfcPhase::Stable.as_ref(), "stable");
    }

    #[test]
    fn test_work_item_status_as_ref() {
        assert_eq!(WorkItemStatus::Queue.as_ref(), "queue");
        assert_eq!(WorkItemStatus::Active.as_ref(), "active");
        assert_eq!(WorkItemStatus::Done.as_ref(), "done");
        assert_eq!(WorkItemStatus::Cancelled.as_ref(), "cancelled");
    }

    #[test]
    fn test_adr_status_as_ref() {
        assert_eq!(AdrStatus::Proposed.as_ref(), "proposed");
        assert_eq!(AdrStatus::Accepted.as_ref(), "accepted");
        assert_eq!(AdrStatus::Superseded.as_ref(), "superseded");
    }

    #[test]
    fn test_checklist_status_as_ref() {
        assert_eq!(ChecklistStatus::Pending.as_ref(), "pending");
        assert_eq!(ChecklistStatus::Done.as_ref(), "done");
        assert_eq!(ChecklistStatus::Cancelled.as_ref(), "cancelled");
    }

    #[test]
    fn test_alternative_status_as_ref() {
        assert_eq!(AlternativeStatus::Considered.as_ref(), "considered");
        assert_eq!(AlternativeStatus::Rejected.as_ref(), "rejected");
        assert_eq!(AlternativeStatus::Accepted.as_ref(), "accepted");
    }

    // =========================================================================
    // AdrEntry/WorkItemEntry accessor Tests
    // =========================================================================

    #[test]
    fn test_adr_entry_meta_accessor() {
        let entry = AdrEntry {
            spec: AdrSpec {
                govctl: AdrMeta {
                    schema: 1,
                    id: "ADR-0001".to_string(),
                    title: "Test ADR".to_string(),
                    status: AdrStatus::Proposed,
                    date: "2026-01-17".to_string(),
                    superseded_by: None,
                    refs: vec![],
                    tags: vec![],
                },
                content: AdrContent::default(),
            },
            path: std::path::PathBuf::from("test.toml"),
        };
        assert_eq!(entry.meta().id, "ADR-0001");
        assert_eq!(entry.meta().title, "Test ADR");
    }

    #[test]
    fn test_work_item_entry_meta_accessor() {
        let entry = WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 1,
                    id: "WI-2026-01-17-001".to_string(),
                    title: "Test Work Item".to_string(),
                    status: WorkItemStatus::Queue,
                    created: Some("2026-01-17".to_string()),
                    started: None,
                    completed: None,
                    refs: vec![],
                    depends_on: vec![],
                    tags: vec![],
                },
                content: WorkItemContent::default(),
                verification: WorkItemVerification::default(),
            },
            path: std::path::PathBuf::from("test.toml"),
        };
        assert_eq!(entry.meta().id, "WI-2026-01-17-001");
        assert_eq!(entry.meta().status, WorkItemStatus::Queue);
    }
}
