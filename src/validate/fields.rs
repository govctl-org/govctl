use crate::cmd::edit::rules as edit_rules;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::find_clause_json;
use crate::model::ClauseStatus;
use crate::write::read_clause;

struct ValidationContext<'a> {
    config: &'a Config,
    /// The artifact being modified (e.g., "RFC-0001:C-NAME").
    artifact_id: &'a str,
}

/// Artifact kinds for validation dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Rfc,
    Clause,
}

impl ArtifactKind {
    fn as_ssot_artifact(self) -> &'static str {
        match self {
            Self::Rfc => "rfc",
            Self::Clause => "clause",
        }
    }
}

#[derive(Debug, Clone)]
enum FieldValidation {
    /// No validation required.
    None,
    /// Must be valid semver (e.g., "1.2.3").
    Semver,
    /// Must be a valid clause reference within same RFC, target must be active.
    ClauseSupersededBy,
    /// Must be a valid artifact reference (RFC-xxx, ADR-xxx, etc.).
    ArtifactRef,
    /// Must be a valid enum value (validated by serde).
    EnumValue,
}

impl FieldValidation {
    /// Get the validation rule for a field.
    fn for_field(kind: ArtifactKind, field: &str) -> Self {
        match edit_rules::field_validation_kind(kind.as_ssot_artifact(), field) {
            Some(edit_rules::ValidationKind::Semver) => Self::Semver,
            Some(edit_rules::ValidationKind::ClauseSupersededBy) => Self::ClauseSupersededBy,
            Some(edit_rules::ValidationKind::ArtifactRef) => Self::ArtifactRef,
            Some(edit_rules::ValidationKind::EnumValue) => Self::EnumValue,
            None => Self::None,
        }
    }

    /// Validate a value.
    fn validate(&self, ctx: &ValidationContext, value: &str) -> anyhow::Result<()> {
        match self {
            Self::None => Ok(()),
            Self::EnumValue => Ok(()),
            Self::Semver => validate_semver(value),
            Self::ClauseSupersededBy => validate_clause_superseded_by(ctx, value),
            Self::ArtifactRef => validate_artifact_ref(ctx, value),
        }
    }
}

/// Validate a field value before setting.
pub fn validate_field(
    config: &Config,
    artifact_id: &str,
    kind: ArtifactKind,
    field: &str,
    value: &str,
) -> anyhow::Result<()> {
    let ctx = ValidationContext {
        config,
        artifact_id,
    };
    let validation = FieldValidation::for_field(kind, field);
    validation.validate(&ctx, value)
}

fn validate_semver(value: &str) -> anyhow::Result<()> {
    semver::Version::parse(value).map_err(|_| {
        Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            format!("Invalid semver: {value}"),
            value,
        )
    })?;
    Ok(())
}

fn validate_clause_superseded_by(ctx: &ValidationContext, target: &str) -> anyhow::Result<()> {
    if target.is_empty() {
        return Ok(());
    }

    let (source_rfc, source_clause) = ctx.artifact_id.split_once(':').ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid clause ID format: {}", ctx.artifact_id),
            ctx.artifact_id,
        )
    })?;
    if source_rfc.is_empty() || source_clause.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid clause ID format: {}", ctx.artifact_id),
            ctx.artifact_id,
        )
        .into());
    }

    let full_target = if target.contains(':') {
        target.to_string()
    } else {
        format!("{source_rfc}:{target}")
    };

    let (target_rfc, target_clause) = full_target.split_once(':').ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid target clause ID: {target}"),
            target,
        )
    })?;
    if target_rfc.is_empty() || target_clause.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid target clause ID: {target}"),
            target,
        )
        .into());
    }

    if target_rfc != source_rfc {
        return Err(Diagnostic::new(
            DiagnosticCode::E0206ClauseSupersededByUnknown,
            format!(
                "superseded_by must reference a clause in the same RFC (got {target_rfc}, expected {source_rfc})"
            ),
            target,
        )
        .into());
    }

    let target_path = find_clause_json(ctx.config, &full_target).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Target clause not found: {full_target}"),
            &full_target,
        )
    })?;

    let target_clause = read_clause(ctx.config, &target_path)?;
    match target_clause.status {
        ClauseStatus::Active => Ok(()),
        ClauseStatus::Superseded => Err(Diagnostic::new(
            DiagnosticCode::E0207ClauseSupersededByNotActive,
            format!("Cannot supersede by a superseded clause: {full_target}"),
            &full_target,
        )
        .into()),
        ClauseStatus::Deprecated => Err(Diagnostic::new(
            DiagnosticCode::E0207ClauseSupersededByNotActive,
            format!("Cannot supersede by a deprecated clause: {full_target}"),
            &full_target,
        )
        .into()),
    }
}

fn validate_artifact_ref(ctx: &ValidationContext, ref_id: &str) -> anyhow::Result<()> {
    use crate::load::find_rfc_json;
    use crate::parse::{load_adrs, load_work_items};

    if ref_id.starts_with("RFC-") {
        if find_rfc_json(ctx.config, ref_id).is_none() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else if ref_id.starts_with("ADR-") {
        let adrs = load_adrs(ctx.config)?;
        if !adrs.iter().any(|a| a.spec.govctl.id == ref_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else if ref_id.starts_with("WI-") {
        let items = load_work_items(ctx.config)?;
        if !items.iter().any(|w| w.spec.govctl.id == ref_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Unknown artifact type: {ref_id}"),
            ref_id,
        )
        .into());
    }

    check_ref_hierarchy(ctx.artifact_id, ref_id, ctx.artifact_id).map_err(|e| e.into())
}

/// Enforce [[RFC-0000:C-REFERENCE-HIERARCHY]] for `refs` targets.
pub(super) fn check_ref_hierarchy(
    artifact_id: &str,
    ref_id: &str,
    diagnostic_path: &str,
) -> Result<(), Diagnostic> {
    let owner_is_rfc = artifact_id.starts_with("RFC-");
    let owner_is_adr = artifact_id.starts_with("ADR-");
    let owner_is_wi = artifact_id.starts_with("WI-");

    if owner_is_wi {
        return Ok(());
    }
    if owner_is_rfc && (ref_id.starts_with("ADR-") || ref_id.starts_with("WI-")) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0112RfcReferenceHierarchy,
            format!(
                "RFC '{artifact_id}' references {ref_id}, but RFCs are higher authority than ADRs and Work Items — remove this reference (the ADR or Work Item should reference the RFC, not the other way around)"
            ),
            diagnostic_path,
        ));
    }
    if owner_is_adr && ref_id.starts_with("WI-") {
        return Err(Diagnostic::new(
            DiagnosticCode::E0306AdrReferenceHierarchy,
            format!(
                "ADR '{artifact_id}' references {ref_id}, but ADRs are higher authority than Work Items — remove this reference (the Work Item should reference the ADR, not the other way around)"
            ),
            diagnostic_path,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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

    // =========================================================================
    // Reference hierarchy ([[RFC-0000:C-REFERENCE-HIERARCHY]])
    // =========================================================================

    #[test]
    fn test_ref_hierarchy_rfc_rejects_adr_and_wi() {
        assert!(check_ref_hierarchy("RFC-0001", "ADR-0001", "f").is_err());
        assert!(check_ref_hierarchy("RFC-0001", "WI-2026-01-01-001", "f").is_err());
    }

    #[test]
    fn test_ref_hierarchy_rfc_allows_rfc_and_clause() {
        assert!(check_ref_hierarchy("RFC-0001", "RFC-0002", "f").is_ok());
        assert!(check_ref_hierarchy("RFC-0001", "RFC-0002:C-FOO", "f").is_ok());
    }

    #[test]
    fn test_ref_hierarchy_adr_rejects_wi() {
        assert!(check_ref_hierarchy("ADR-0001", "WI-2026-01-01-001", "f").is_err());
    }

    #[test]
    fn test_ref_hierarchy_adr_allows_rfc_adr() {
        assert!(check_ref_hierarchy("ADR-0001", "RFC-0000:C-RFC-DEF", "f").is_ok());
        assert!(check_ref_hierarchy("ADR-0001", "ADR-0002", "f").is_ok());
    }

    #[test]
    fn test_ref_hierarchy_work_allows_any() {
        assert!(check_ref_hierarchy("WI-2026-01-01-001", "WI-2026-01-01-002", "f").is_ok());
        assert!(check_ref_hierarchy("WI-2026-01-01-001", "ADR-0001", "f").is_ok());
    }
}
