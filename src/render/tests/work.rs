use super::*;

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
