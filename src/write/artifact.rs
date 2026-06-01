//! RFC and clause artifact read/write helpers.

use super::{WriteOp, write_file};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{ClauseSpec, ClauseWire, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_json_value, validate_toml_value, with_schema_header};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

const RFC_METADATA_KEYS: &[&str] = &[
    "title",
    "version",
    "status",
    "phase",
    "owners",
    "created",
    "updated",
    "supersedes",
    "refs",
    "signature",
];

const CLAUSE_METADATA_KEYS: &[&str] = &[
    "title",
    "kind",
    "status",
    "anchors",
    "superseded_by",
    "since",
];

struct ArtifactIo {
    read_label: &'static str,
    message_label: &'static str,
    schema: ArtifactSchema,
    schema_error: DiagnosticCode,
    normalize_toml: fn(&mut toml::Value),
    normalize_json: fn(&mut serde_json::Value),
}

const RFC_IO: ArtifactIo = ArtifactIo {
    read_label: "RFC",
    message_label: "RFC",
    schema: ArtifactSchema::Rfc,
    schema_error: DiagnosticCode::E0101RfcSchemaInvalid,
    normalize_toml: normalize_rfc_value,
    normalize_json: normalize_rfc_json,
};

const CLAUSE_IO: ArtifactIo = ArtifactIo {
    read_label: "clause",
    message_label: "clause",
    schema: ArtifactSchema::Clause,
    schema_error: DiagnosticCode::E0201ClauseSchemaInvalid,
    normalize_toml: normalize_clause_value,
    normalize_json: normalize_clause_json,
};

fn read_artifact<Wire, Spec>(
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

fn write_toml_artifact<Wire: Serialize>(
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

/// Read RFC from file and validate its normalized structure.
/// Handles both legacy flat format and new `[govctl]` wire format (TOML and JSON).
pub fn read_rfc(config: &Config, path: &Path) -> DiagnosticResult<RfcSpec> {
    read_artifact::<RfcWire, RfcSpec>(config, path, &RFC_IO)
}

/// Write RFC to file in TOML only.
/// TOML output uses the `[govctl]` wire format plus schema header.
pub fn write_rfc(
    path: &Path,
    rfc: &RfcSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    let wire: RfcWire = rfc.clone().into();
    write_toml_artifact(
        path,
        &wire,
        ArtifactSchema::Rfc,
        DiagnosticCode::E0101RfcSchemaInvalid,
        "RFC",
        op,
        display_path,
    )
}

/// Read clause from file and validate its normalized structure.
/// Handles both legacy flat format and new `[govctl]` + `[content]` wire format.
pub fn read_clause(config: &Config, path: &Path) -> DiagnosticResult<ClauseSpec> {
    read_artifact::<ClauseWire, ClauseSpec>(config, path, &CLAUSE_IO)
}

/// Write clause to file in TOML only.
/// TOML output uses the `[govctl]` + `[content]` wire format plus schema header.
pub fn write_clause(
    path: &Path,
    clause: &ClauseSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    let wire: ClauseWire = clause.clone().into();
    write_toml_artifact(
        path,
        &wire,
        ArtifactSchema::Clause,
        DiagnosticCode::E0201ClauseSchemaInvalid,
        "clause",
        op,
        display_path,
    )
}

fn move_toml_keys(
    source: &mut toml::map::Map<String, toml::Value>,
    target: &mut toml::map::Map<String, toml::Value>,
    keys: &[&str],
) {
    for key in keys {
        if let Some(v) = source.remove(*key) {
            target.insert(key.to_string(), v);
        }
    }
}

fn extract_toml_govctl(
    root: &mut toml::map::Map<String, toml::Value>,
    id_key: &str,
    metadata_keys: &[&str],
) -> Option<toml::Value> {
    if root.contains_key("govctl") {
        return None;
    }
    let id = root.remove(id_key)?;

    let mut govctl = toml::map::Map::new();
    govctl.insert("schema".to_string(), toml::Value::Integer(1));
    govctl.insert("id".to_string(), id);
    move_toml_keys(root, &mut govctl, metadata_keys);
    Some(toml::Value::Table(govctl))
}

/// Normalize a flat RFC TOML value into the `[govctl]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_rfc_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    if let Some(govctl) = extract_toml_govctl(root, "rfc_id", RFC_METADATA_KEYS) {
        root.insert("govctl".to_string(), govctl);
    }
}

/// Normalize a flat clause TOML value into the `[govctl]` + `[content]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_clause_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    let Some(govctl) = extract_toml_govctl(root, "clause_id", CLAUSE_METADATA_KEYS) else {
        return;
    };
    root.insert("govctl".to_string(), govctl);

    let mut content = toml::map::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), toml::Value::Table(content));
}

fn move_json_keys(
    source: &mut serde_json::Map<String, serde_json::Value>,
    target: &mut serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) {
    for key in keys {
        if let Some(v) = source.remove(*key) {
            target.insert(key.to_string(), v);
        }
    }
}

fn extract_json_govctl(
    root: &mut serde_json::Map<String, serde_json::Value>,
    id_key: &str,
    metadata_keys: &[&str],
) -> Option<serde_json::Value> {
    if root.contains_key("govctl") {
        return None;
    }
    let id = root.remove(id_key)?;
    let mut govctl = serde_json::Map::new();
    govctl.insert("schema".to_string(), serde_json::json!(1));
    govctl.insert("id".to_string(), id);
    move_json_keys(root, &mut govctl, metadata_keys);
    Some(serde_json::Value::Object(govctl))
}

/// Normalize a flat RFC JSON value into the `govctl` wire layout.
pub(crate) fn normalize_rfc_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    if let Some(govctl) = extract_json_govctl(root, "rfc_id", RFC_METADATA_KEYS) {
        root.insert("govctl".to_string(), govctl);
    }
}

/// Normalize a flat clause JSON value into the `govctl` + `content` wire layout.
pub(crate) fn normalize_clause_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    let Some(govctl) = extract_json_govctl(root, "clause_id", CLAUSE_METADATA_KEYS) else {
        return;
    };
    root.insert("govctl".to_string(), govctl);

    let mut content = serde_json::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), serde_json::Value::Object(content));
}
