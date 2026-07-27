use super::*;

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
