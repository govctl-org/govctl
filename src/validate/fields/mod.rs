use crate::cmd::edit::rules as edit_rules;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::find_clause_json;
use crate::model::ClauseStatus;
use crate::validate::reference_hierarchy::{ReferenceSurface, check_ref_hierarchy};
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

    check_ref_hierarchy(
        ctx.artifact_id,
        ref_id,
        ctx.artifact_id,
        ReferenceSurface::StructuredRef,
    )
    .map_err(|e| e.into())
}

#[cfg(test)]
mod tests;
