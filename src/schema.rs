//! Runtime JSON Schema validation for governance artifacts.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use serde_json::Value;
use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::Path;

pub struct SchemaTemplate {
    pub filename: &'static str,
    pub content: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub enum ArtifactSchema {
    Rfc,
    Clause,
    Adr,
    WorkItem,
    Release,
    Guard,
}

impl ArtifactSchema {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Rfc => "rfc.schema.json",
            Self::Clause => "clause.schema.json",
            Self::Adr => "adr.schema.json",
            Self::WorkItem => "work.schema.json",
            Self::Release => "release.schema.json",
            Self::Guard => "guard.schema.json",
        }
    }

    fn bundled_content(self) -> &'static str {
        match self {
            Self::Rfc => include_str!("../gov/schema/rfc.schema.json"),
            Self::Clause => include_str!("../gov/schema/clause.schema.json"),
            Self::Adr => include_str!("../gov/schema/adr.schema.json"),
            Self::WorkItem => include_str!("../gov/schema/work.schema.json"),
            Self::Release => include_str!("../gov/schema/release.schema.json"),
            Self::Guard => include_str!("../gov/schema/guard.schema.json"),
        }
    }

    fn diagnostic_code(self) -> DiagnosticCode {
        match self {
            Self::Rfc => DiagnosticCode::E0101RfcSchemaInvalid,
            Self::Clause => DiagnosticCode::E0201ClauseSchemaInvalid,
            Self::Adr => DiagnosticCode::E0301AdrSchemaInvalid,
            Self::WorkItem => DiagnosticCode::E0401WorkSchemaInvalid,
            Self::Release => DiagnosticCode::E0704ReleaseSchemaInvalid,
            Self::Guard => DiagnosticCode::E1001GuardSchemaInvalid,
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Rfc => "RFC",
            Self::Clause => "clause",
            Self::Adr => "ADR",
            Self::WorkItem => "work item",
            Self::Release => "release",
            Self::Guard => "verification guard",
        }
    }

    /// Deterministic relative path from an artifact's canonical location to its JSON Schema.
    /// Paths are fixed because the directory layout under `gov/` is not configurable.
    pub fn relative_schema_path(self) -> &'static str {
        match self {
            Self::Rfc => "../../schema/rfc.schema.json",
            Self::Clause => "../../../schema/clause.schema.json",
            Self::Adr => "../schema/adr.schema.json",
            Self::WorkItem => "../schema/work.schema.json",
            Self::Release => "schema/release.schema.json",
            Self::Guard => "../schema/guard.schema.json",
        }
    }
}

/// Prepend a `#:schema` comment header to TOML content for IDE schema association.
pub fn with_schema_header(kind: ArtifactSchema, body: &str) -> String {
    format!("#:schema {}\n\n{}", kind.relative_schema_path(), body)
}

pub const ARTIFACT_SCHEMA_TEMPLATES: &[SchemaTemplate] = &[
    SchemaTemplate {
        filename: "rfc.schema.json",
        content: include_str!("../gov/schema/rfc.schema.json"),
    },
    SchemaTemplate {
        filename: "clause.schema.json",
        content: include_str!("../gov/schema/clause.schema.json"),
    },
    SchemaTemplate {
        filename: "adr.schema.json",
        content: include_str!("../gov/schema/adr.schema.json"),
    },
    SchemaTemplate {
        filename: "work.schema.json",
        content: include_str!("../gov/schema/work.schema.json"),
    },
    SchemaTemplate {
        filename: "release.schema.json",
        content: include_str!("../gov/schema/release.schema.json"),
    },
    SchemaTemplate {
        filename: "guard.schema.json",
        content: include_str!("../gov/schema/guard.schema.json"),
    },
];

pub fn validate_json_value(
    kind: ArtifactSchema,
    config: &Config,
    artifact_path: &Path,
    value: &Value,
) -> Result<(), Diagnostic> {
    validate_value(kind, config, artifact_path, value)
}

pub fn validate_toml_value(
    kind: ArtifactSchema,
    config: &Config,
    artifact_path: &Path,
    value: &toml::Value,
) -> Result<(), Diagnostic> {
    let json_value = serde_json::to_value(value).map_err(|err| {
        Diagnostic::new(
            kind.diagnostic_code(),
            format!(
                "Failed to normalize parsed {} for schema validation: {}",
                kind.display_name(),
                err
            ),
            config.display_path(artifact_path).display().to_string(),
        )
    })?;
    validate_value(kind, config, artifact_path, &json_value)
}

fn validate_value(
    kind: ArtifactSchema,
    config: &Config,
    artifact_path: &Path,
    value: &Value,
) -> Result<(), Diagnostic> {
    let artifact_display = config.display_path(artifact_path).display().to_string();
    let schema_path = config.schema_dir().join(kind.filename());
    let schema_display = config.display_path(&schema_path).display().to_string();

    let schema_text = match std::fs::read_to_string(&schema_path) {
        Ok(text) => Cow::Owned(text),
        Err(err) if err.kind() == ErrorKind::NotFound => Cow::Borrowed(kind.bundled_content()),
        Err(err) => {
            return Err(Diagnostic::new(
                kind.diagnostic_code(),
                format!("Failed to read schema file '{}': {}", schema_display, err),
                schema_display,
            ));
        }
    };

    let schema_value: Value = serde_json::from_str(&schema_text).map_err(|err| {
        Diagnostic::new(
            kind.diagnostic_code(),
            format!("Invalid schema file '{}': {}", schema_display, err),
            schema_display.clone(),
        )
    })?;

    let compiled = jsonschema::validator_for(&schema_value).map_err(|err| {
        Diagnostic::new(
            kind.diagnostic_code(),
            format!("Failed to compile schema '{}': {}", schema_display, err),
            schema_display.clone(),
        )
    })?;

    let mut violations: Vec<String> = compiled
        .iter_errors(value)
        .map(|err| err.to_string())
        .collect();
    violations.sort();
    violations.dedup();

    if violations.is_empty() {
        return Ok(());
    }

    let body = violations
        .into_iter()
        .map(|item| format!("  - {item}"))
        .collect::<Vec<_>>()
        .join("\n");

    Err(Diagnostic::new(
        kind.diagnostic_code(),
        format!(
            "{} does not match schema '{}':\n{}",
            kind.display_name(),
            kind.filename(),
            body
        ),
        artifact_display,
    ))
}
