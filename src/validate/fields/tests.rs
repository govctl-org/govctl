use super::*;

// =========================================================================
// FieldValidation::for_field Tests
// =========================================================================

#[test]
fn test_field_validation_clause_since() {
    assert!(matches!(
        FieldValidation::for_field(ArtifactKind::Clause, "since"),
        FieldValidation::Semver
    ));
}

#[test]
fn test_field_validation_clause_superseded_by() {
    assert!(matches!(
        FieldValidation::for_field(ArtifactKind::Clause, "superseded_by"),
        FieldValidation::ClauseSupersededBy
    ));
}

#[test]
fn test_field_validation_rfc_version() {
    assert!(matches!(
        FieldValidation::for_field(ArtifactKind::Rfc, "version"),
        FieldValidation::Semver
    ));
}

#[test]
fn test_field_validation_unknown_field() {
    assert!(matches!(
        FieldValidation::for_field(ArtifactKind::Rfc, "unknown"),
        FieldValidation::None
    ));
}
