use super::*;

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
