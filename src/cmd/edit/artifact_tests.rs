use super::ArtifactType;

#[test]
fn test_artifact_type_clause() {
    assert_eq!(
        ArtifactType::from_id("RFC-0001:C-NAME"),
        Some(ArtifactType::Clause)
    );
    assert_eq!(
        ArtifactType::from_id("RFC-0000:C-SUMMARY"),
        Some(ArtifactType::Clause)
    );
}

#[test]
fn test_artifact_type_rfc() {
    assert_eq!(ArtifactType::from_id("RFC-0001"), Some(ArtifactType::Rfc));
    assert_eq!(ArtifactType::from_id("RFC-9999"), Some(ArtifactType::Rfc));
}

#[test]
fn test_artifact_type_adr() {
    assert_eq!(ArtifactType::from_id("ADR-0001"), Some(ArtifactType::Adr));
    assert_eq!(ArtifactType::from_id("ADR-0007"), Some(ArtifactType::Adr));
}

#[test]
fn test_artifact_type_work_item_by_prefix() {
    assert_eq!(
        ArtifactType::from_id("WI-2026-01-17-001"),
        Some(ArtifactType::WorkItem)
    );
}

#[test]
fn test_artifact_type_work_item_by_hyphen() {
    // Any ID with hyphen that doesn't match RFC/ADR/Clause is WorkItem.
    assert_eq!(
        ArtifactType::from_id("2026-01-17-add-tests"),
        Some(ArtifactType::WorkItem)
    );
}

#[test]
fn test_artifact_type_unknown() {
    assert_eq!(ArtifactType::from_id("UNKNOWN"), None);
    assert_eq!(ArtifactType::from_id("foo"), None);
}
