use super::*;
use crate::model::{
    AdrContent, AdrEntry, AdrMeta, AdrSpec, AdrStatus, Alternative, AlternativeStatus,
    ChangelogCategory, ChecklistItem, ChecklistStatus, JournalEntry, WorkItemContent,
    WorkItemEntry, WorkItemMeta, WorkItemSpec, WorkItemStatus,
};

const DEFAULT_PATTERN: &str = r"\[\[(RFC-\d{4}(?::C-[A-Z][A-Z0-9-]*)?|ADR-\d{4}|WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3}))\]\]";

#[test]
fn test_expand_inline_refs_rfc() {
    let text = "See [[RFC-0000]] for details.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(result, "See [RFC-0000](../rfc/RFC-0000.md) for details.");
}

#[test]
fn test_expand_inline_refs_clause() {
    let text = "Per [[RFC-0000:C-WORK-DEF]], work items must...";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        "Per [RFC-0000:C-WORK-DEF](../rfc/RFC-0000.md#rfc-0000c-work-def), work items must..."
    );
}

#[test]
fn test_expand_inline_refs_adr() {
    let text = "This follows [[ADR-0005]] guidelines.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        "This follows [ADR-0005](../adr/ADR-0005.md) guidelines."
    );
}

#[test]
fn test_expand_inline_refs_multiple() {
    let text = "See [[RFC-0000]] and [[ADR-0042]] for context.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        "See [RFC-0000](../rfc/RFC-0000.md) and [ADR-0042](../adr/ADR-0042.md) for context."
    );
}

#[test]
fn test_expand_inline_refs_no_match() {
    let text = "No references here.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(result, "No references here.");
}

#[test]
fn test_expand_inline_refs_invalid_pattern() {
    let text = "[[RFC-0000]] test";
    let result = expand_inline_refs(text, "[invalid(regex");
    // Invalid pattern returns text unchanged
    assert_eq!(result, "[[RFC-0000]] test");
}

#[test]
fn test_ref_link_from_root_rfc() {
    let result = ref_link_from_root("RFC-0000", "docs");
    assert_eq!(result, "[RFC-0000](docs/rfc/RFC-0000.md)");
}

#[test]
fn test_ref_link_from_root_clause() {
    let result = ref_link_from_root("RFC-0000:C-WORK-DEF", "docs");
    assert_eq!(
        result,
        "[RFC-0000:C-WORK-DEF](docs/rfc/RFC-0000.md#rfc-0000c-work-def)"
    );
}

#[test]
fn test_ref_link_from_root_adr() {
    let result = ref_link_from_root("ADR-0005", "docs");
    assert_eq!(result, "[ADR-0005](docs/adr/ADR-0005.md)");
}

#[test]
fn test_ref_link_from_root_custom_path() {
    let result = ref_link_from_root("RFC-0001", "documentation");
    assert_eq!(result, "[RFC-0001](documentation/rfc/RFC-0001.md)");
}

#[test]
fn test_expand_inline_refs_from_root() {
    let text = "Per [[RFC-0002:C-RESOURCE-MODEL]], resources use verb pattern.";
    let result = expand_inline_refs_from_root(text, DEFAULT_PATTERN, "docs");
    assert_eq!(
        result,
        "Per [RFC-0002:C-RESOURCE-MODEL](docs/rfc/RFC-0002.md#rfc-0002c-resource-model), resources use verb pattern."
    );
}

#[test]
fn test_expand_inline_refs_from_root_multiple() {
    let text = "See [[RFC-0000]] and [[ADR-0018]] for details.";
    let result = expand_inline_refs_from_root(text, DEFAULT_PATTERN, "docs");
    assert_eq!(
        result,
        "See [RFC-0000](docs/rfc/RFC-0000.md) and [ADR-0018](docs/adr/ADR-0018.md) for details."
    );
}

// Work item inline reference tests (per ADR-0020 ID formats)
// Constructs strings at runtime to avoid source_scan matching test fixtures

fn wi_ref(id: &str) -> String {
    format!("[[{}]]", id)
}

#[test]
fn test_expand_inline_refs_work_item_sequential() {
    let id = "WI-9999-01-26-001";
    let text = format!("See {} for task details.", wi_ref(id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!("See [{}](../work/{}.md) for task details.", id, id)
    );
}

#[test]
fn test_expand_inline_refs_work_item_author_hash() {
    let id = "WI-9999-01-26-a7f3-001";
    let text = format!("See {} for task details.", wi_ref(id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!("See [{}](../work/{}.md) for task details.", id, id)
    );
}

#[test]
fn test_expand_inline_refs_work_item_random() {
    let id = "WI-9999-01-26-b2c9";
    let text = format!("See {} for task details.", wi_ref(id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!("See [{}](../work/{}.md) for task details.", id, id)
    );
}

#[test]
fn test_expand_inline_refs_work_item_mixed() {
    let wi_id = "WI-9999-01-26-001";
    let text = format!("Per [[RFC-0000]], see {} and [[ADR-0020]].", wi_ref(wi_id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!(
            "Per [RFC-0000](../rfc/RFC-0000.md), see [{}](../work/{}.md) and [ADR-0020](../adr/ADR-0020.md).",
            wi_id, wi_id
        )
    );
}

// Tests for render_adr with new Alternative fields per [[ADR-0027]]

#[test]
fn test_render_adr_alternatives_with_pros_cons() -> Result<(), Box<dyn std::error::Error>> {
    let adr = AdrEntry {
        spec: AdrSpec {
            govctl: AdrMeta::new("ADR-9999", "Test ADR", AdrStatus::Accepted, "2026-02-22"),
            content: AdrContent {
                context: "Test context".to_string(),
                decision: "Test decision".to_string(),
                consequences: "Test consequences".to_string(),
                alternatives: vec![Alternative {
                    text: "Option A".to_string(),
                    status: AlternativeStatus::Considered,
                    pros: vec!["Fast".to_string(), "Cheap".to_string()],
                    cons: vec!["Less reliable".to_string()],
                    rejection_reason: None,
                }],
            },
        },
        path: std::path::PathBuf::new(),
    };

    let result = render_adr(&adr)?;
    assert!(result.contains("### Option A"));
    assert!(result.contains("- **Pros:** Fast, Cheap"));
    assert!(result.contains("- **Cons:** Less reliable"));
    Ok(())
}

#[test]
fn test_render_adr_alternatives_rejected_with_reason() -> Result<(), Box<dyn std::error::Error>> {
    let adr = AdrEntry {
        spec: AdrSpec {
            govctl: AdrMeta::new(
                "ADR-9998",
                "Test ADR Rejected",
                AdrStatus::Accepted,
                "2026-02-22",
            ),
            content: AdrContent {
                context: "Test context".to_string(),
                decision: "Test decision".to_string(),
                consequences: "Test consequences".to_string(),
                alternatives: vec![Alternative {
                    text: "Option B".to_string(),
                    status: AlternativeStatus::Rejected,
                    pros: vec![],
                    cons: vec!["Too expensive".to_string()],
                    rejection_reason: Some("Budget constraints".to_string()),
                }],
            },
        },
        path: std::path::PathBuf::new(),
    };

    let result = render_adr(&adr)?;
    assert!(result.contains("### Option B (rejected)"));
    assert!(result.contains("- **Rejected because:** Budget constraints"));
    Ok(())
}

// Tests for render_work_item with legacy inline history rendering per [[ADR-0047]]

#[test]
fn test_render_work_item_journal() -> Result<(), Box<dyn std::error::Error>> {
    let mut meta = WorkItemMeta::new(
        "WI-2026-02-22-001",
        "Test Work Item",
        WorkItemStatus::Active,
    );
    meta.created = Some("2026-02-22".to_string());
    meta.started = Some("2026-02-22".to_string());

    let item = WorkItemEntry {
        spec: WorkItemSpec {
            govctl: meta,
            content: WorkItemContent {
                description: "Test description".to_string(),
                journal: vec![JournalEntry {
                    date: "2026-02-22".to_string(),
                    scope: None,
                    content: "Started implementation".to_string(),
                }],
                acceptance_criteria: vec![],
                notes: vec![],
            },
            verification: crate::model::WorkItemVerification::default(),
        },
        path: std::path::PathBuf::new(),
    };

    let result = render_work_item(&item)?;
    assert!(result.contains("## Journal"));
    assert!(result.contains("Legacy execution history"));
    assert!(result.contains("loop state"));
    assert!(result.contains("### 2026-02-22"));
    assert!(result.contains("Started implementation"));
    Ok(())
}

#[test]
fn test_render_work_item_journal_with_scope() -> Result<(), Box<dyn std::error::Error>> {
    let mut meta = WorkItemMeta::new(
        "WI-2026-02-22-002",
        "Test Work Item with Scope",
        WorkItemStatus::Active,
    );
    meta.created = Some("2026-02-22".to_string());
    meta.started = Some("2026-02-22".to_string());

    let item = WorkItemEntry {
        spec: WorkItemSpec {
            govctl: meta,
            content: WorkItemContent {
                description: "Test description".to_string(),
                journal: vec![
                    JournalEntry {
                        date: "2026-02-22".to_string(),
                        scope: Some("API".to_string()),
                        content: "Created endpoint".to_string(),
                    },
                    JournalEntry {
                        date: "2026-02-23".to_string(),
                        scope: Some("Testing".to_string()),
                        content: "Added unit tests".to_string(),
                    },
                ],
                acceptance_criteria: vec![],
                notes: vec![],
            },
            verification: crate::model::WorkItemVerification::default(),
        },
        path: std::path::PathBuf::new(),
    };

    let result = render_work_item(&item)?;
    assert!(result.contains("### 2026-02-22 · API"));
    assert!(result.contains("Created endpoint"));
    assert!(result.contains("### 2026-02-23 · Testing"));
    assert!(result.contains("Added unit tests"));
    Ok(())
}

#[test]
fn test_render_work_item_acceptance_criteria_show_categories()
-> Result<(), Box<dyn std::error::Error>> {
    let mut done = ChecklistItem::with_category("Fix rendered category", ChangelogCategory::Fixed);
    done.status = ChecklistStatus::Done;
    let mut cancelled =
        ChecklistItem::with_category("Obsolete validation path", ChangelogCategory::Chore);
    cancelled.status = ChecklistStatus::Cancelled;

    let mut meta = WorkItemMeta::new(
        "WI-2026-02-22-003",
        "Test Work Item Categories",
        WorkItemStatus::Active,
    );
    meta.created = Some("2026-02-22".to_string());
    meta.started = Some("2026-02-22".to_string());

    let item = WorkItemEntry {
        spec: WorkItemSpec {
            govctl: meta,
            content: WorkItemContent {
                description: "Test description".to_string(),
                journal: vec![],
                acceptance_criteria: vec![
                    ChecklistItem::with_category("Add reviewer context", ChangelogCategory::Added),
                    done,
                    cancelled,
                ],
                notes: vec![],
            },
            verification: crate::model::WorkItemVerification::default(),
        },
        path: std::path::PathBuf::new(),
    };

    let result = render_work_item(&item)?;
    assert!(result.contains("- [ ] added: Add reviewer context"));
    assert!(result.contains("- [x] fixed: Fix rendered category"));
    assert!(result.contains("- ~~chore: Obsolete validation path~~"));
    Ok(())
}
