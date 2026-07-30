use super::{WriteOp, write_file};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::schema::{ArtifactSchema, validate_toml_value, with_schema_header};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) struct ArtifactIo {
    pub read_label: &'static str,
    pub message_label: &'static str,
    pub schema: ArtifactSchema,
    pub schema_error: DiagnosticCode,
}

pub(super) fn read_artifact<Wire, Spec>(
    config: &Config,
    path: &Path,
    io: &ArtifactIo,
) -> DiagnosticResult<Spec>
where
    Wire: DeserializeOwned + Into<Spec>,
{
    let content = std::fs::read_to_string(path).map_err(|err| {
        Diagnostic::io_error(
            format!("read {}", io.read_label),
            err,
            path.display().to_string(),
        )
    })?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        return Err(Diagnostic::new(
            DiagnosticCode::E0505MigrationRequired,
            "Legacy RFC/clause JSON artifact storage is unsupported. Migrate this repository with a compatible earlier govctl version before upgrading.",
            path.display().to_string(),
        ));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
        return Err(Diagnostic::new(
            io.schema_error,
            "Unsupported artifact source extension; expected TOML",
            path.display().to_string(),
        ));
    }

    let raw: toml::Value = toml::from_str(&content).map_err(|err| {
        Diagnostic::new(
            io.schema_error,
            format!("Failed to parse {} TOML: {err}", io.message_label),
            path.display().to_string(),
        )
    })?;
    validate_toml_value(io.schema, config, path, &raw)?;
    let wire: Wire = raw.try_into().map_err(|err| {
        Diagnostic::new(
            io.schema_error,
            format!("Failed to deserialize {} TOML: {err}", io.message_label),
            path.display().to_string(),
        )
    })?;
    Ok(wire.into())
}

pub(super) fn write_toml_artifact<Wire: Serialize>(
    config: &Config,
    path: &Path,
    wire: &Wire,
    io: &ArtifactIo,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    let diagnostic_path = display_path.unwrap_or(path);
    let body = toml::to_string_pretty(wire).map_err(|err| {
        Diagnostic::new(
            io.schema_error,
            format!("Failed to serialize {} TOML: {err}", io.message_label),
            diagnostic_path.display().to_string(),
        )
    })?;
    let raw = toml::from_str(&body).map_err(|err| {
        Diagnostic::new(
            io.schema_error,
            format!("Failed to normalize {} TOML: {err}", io.message_label),
            diagnostic_path.display().to_string(),
        )
    })?;
    validate_toml_value(io.schema, config, path, &raw)?;
    let content = with_schema_header(io.schema, &body);
    write_file(path, &content, op, display_path)
}
