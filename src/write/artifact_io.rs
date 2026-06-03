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
    pub normalize_toml: fn(&mut toml::Value),
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
            "Legacy RFC/clause JSON artifact storage is no longer supported. Use govctl <0.9 to run `govctl migrate` before upgrading.",
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

    let mut raw: toml::Value = toml::from_str(&content).map_err(|err| {
        Diagnostic::new(
            io.schema_error,
            format!("Failed to parse {} TOML: {err}", io.message_label),
            path.display().to_string(),
        )
    })?;
    (io.normalize_toml)(&mut raw);
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
    path: &Path,
    wire: &Wire,
    schema: ArtifactSchema,
    schema_error: DiagnosticCode,
    message_label: &str,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    let diagnostic_path = display_path.unwrap_or(path);
    let body = toml::to_string_pretty(wire).map_err(|err| {
        Diagnostic::new(
            schema_error,
            format!("Failed to serialize {message_label} TOML: {err}"),
            diagnostic_path.display().to_string(),
        )
    })?;
    let content = with_schema_header(schema, &body);
    write_file(path, &content, op, display_path)
}
