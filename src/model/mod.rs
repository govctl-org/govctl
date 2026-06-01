//! Data models for all governed artifacts.
//!
//! Implements data structures per [[RFC-0000]] governance framework:
//! - RFCs with clauses ([[RFC-0000:C-RFC-DEF]])
//! - ADRs ([[RFC-0000:C-ADR-DEF]])
//! - Work Items ([[RFC-0000:C-WORK-DEF]])
//!
//! Lifecycle state machines per [[RFC-0001]].

mod adr;
mod changelog;
mod clause;
mod guard;
mod index;
mod release;
mod rfc;
#[cfg(test)]
mod tests;
mod work;

pub use adr::{AdrContent, AdrMeta, AdrSpec, AdrStatus, Alternative, AlternativeStatus};
pub use changelog::{ChangelogCategory, ChangelogEntry};
pub use clause::{ClauseKind, ClauseSpec, ClauseStatus, ClauseWire};
pub use guard::{GuardCheck, GuardMeta, GuardSpec};
pub use index::{AdrEntry, ClauseEntry, GuardEntry, ProjectIndex, RfcIndex, WorkItemEntry};
pub use release::{Release, ReleasesFile};
pub use rfc::{RfcPhase, RfcSpec, RfcStatus, RfcWire, SectionSpec};
#[cfg(test)]
pub use work::JournalEntry;
pub use work::{
    ChecklistItem, ChecklistStatus, WorkItemContent, WorkItemMeta, WorkItemSpec, WorkItemStatus,
    WorkItemVerification,
};
