use super::*;
use crate::model::{
    AdrContent, AdrEntry, AdrMeta, AdrSpec, AdrStatus, Alternative, AlternativeStatus,
    ChangelogCategory, ChecklistItem, ChecklistStatus, JournalEntry, WorkItemContent,
    WorkItemEntry, WorkItemMeta, WorkItemSpec, WorkItemStatus,
};

const DEFAULT_PATTERN: &str = r"\[\[(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3}))\]\]";

// Work item inline reference tests (per ADR-0020 ID formats)
// Constructs strings at runtime to avoid source_scan matching test fixtures

fn wi_ref(id: &str) -> String {
    format!("[[{}]]", id)
}

mod adr;
mod links;
mod rfc;
mod work;
