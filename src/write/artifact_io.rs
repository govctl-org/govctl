use super::{WriteOp, write_file};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::schema::{ArtifactSchema, validate_json_value, validate_toml_value, with_schema_header};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) struct ArtifactIo {
    pub read_label: &'static str,
    pub message_label: &'static str,
    pub schema: ArtifactSchema,
    pub schema_error: DiagnosticCode,
    pub normalize_toml: fn(&mut toml::Value),
    pub normalize_json: fn(&mut serde_json::Value),
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
    let spec = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
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
            wire.into()
        }
        _ => {
            let mut raw: serde_json::Value = serde_json::from_str(&content).map_err(|err| {
                Diagnostic::new(
                    DiagnosticCode::E0902JsonParseError,
                    format!("Failed to parse {} JSON: {err}", io.message_label),
                    path.display().to_string(),
                )
            })?;
            (io.normalize_json)(&mut raw);
            validate_json_value(io.schema, config, path, &raw)?;
            let wire: Wire = serde_json::from_value(raw).map_err(|err| {
                Diagnostic::new(
                    io.schema_error,
                    format!("Failed to deserialize {} JSON: {err}", io.message_label),
                    path.display().to_string(),
                )
            })?;
            wire.into()
        }
    };
    Ok(spec)
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
