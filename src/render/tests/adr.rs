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

#[test]
fn test_superseded_adr_current_projection_is_metadata_only_but_archive_is_complete()
-> Result<(), Box<dyn std::error::Error>> {
    let mut meta = AdrMeta::new(
        "ADR-9997",
        "Historical decision",
        AdrStatus::Superseded,
        "2026-02-22",
    );
    meta.superseded_by = Some("ADR-9998".to_string());
    meta.tags = vec!["cli".to_string()];
    meta.refs = vec!["RFC-0002".to_string()];
    let adr = AdrEntry {
        spec: AdrSpec {
            govctl: meta,
            content: AdrContent {
                context: "Historical context".to_string(),
                decision: "Historical decision body".to_string(),
                consequences: "Historical consequences".to_string(),
                alternatives: vec![],
            },
        },
        path: std::path::PathBuf::new(),
    };

    let current = render_adr_with_projection(&adr, RenderProjection::Current)?;
    let archive = render_adr_with_projection(&adr, RenderProjection::Archive)?;

    assert!(current.contains("# ADR-9997: Historical decision"));
    assert!(current.contains("**Status:** superseded"));
    assert!(current.contains("**Superseded by:** ADR-9998"));
    assert!(!current.contains("Historical context"));
    assert!(!current.contains("Historical decision body"));
    assert!(archive.contains("Historical context"));
    assert!(archive.contains("Historical decision body"));
    Ok(())
}
