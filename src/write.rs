//! JSON and frontmatter mutation utilities.
//!
//! Implements [[ADR-0006]] global dry-run support for content-modifying commands.
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::config::Config;
use crate::model::{ClauseSpec, ClauseWire, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_json_value, validate_toml_value, with_schema_header};
use crate::ui;
use anyhow::{Context, Result};
use std::path::Path;

mod changelog;

pub use changelog::{BumpLevel, ParsedChange, add_changelog_change, bump_rfc_version, today};

pub fn parse_changelog_change(change: &str) -> Result<ParsedChange> {
    changelog::parse_changelog_change(change)
}

/// Write operation mode.
///
/// Controls whether write operations execute or just preview.
#[derive(Debug, Clone, Copy, Default)]
pub enum WriteOp {
    /// Actually write to disk
    #[default]
    Execute,
    /// Preview only: show what would be written
    Preview,
}

impl WriteOp {
    /// Create WriteOp from dry_run boolean flag
    pub fn from_dry_run(dry_run: bool) -> Self {
        if dry_run {
            WriteOp::Preview
        } else {
            WriteOp::Execute
        }
    }

    /// Returns true if this is a preview/dry-run operation
    pub fn is_preview(&self) -> bool {
        matches!(self, WriteOp::Preview)
    }
}

/// Write content to a file, respecting WriteOp mode.
///
/// In Preview mode, shows what would be written instead of writing.
/// If `display_path` is provided, it's used for the preview output instead of `path`.
pub fn write_file(
    path: &Path,
    content: &str,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::write(path, content)?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(output_path, content);
        }
    }
    Ok(())
}

/// Create a directory, respecting WriteOp mode.
///
/// In Preview mode, shows what directory would be created.
/// If `display_path` is provided, it's used for the preview output instead of `path`.
pub fn create_dir_all(path: &Path, op: WriteOp, display_path: Option<&Path>) -> Result<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::create_dir_all(path)?;
        }
        WriteOp::Preview => {
            ui::dry_run_mkdir(output_path);
        }
    }
    Ok(())
}

/// Delete a file, respecting WriteOp mode.
///
/// In Preview mode, shows what would be deleted instead of deleting.
/// If `display_path` is provided, it's used for error messages and preview output.
pub fn delete_file(path: &Path, op: WriteOp, display_path: Option<&Path>) -> Result<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to delete file: {}", output_path.display()))?;
        }
        WriteOp::Preview => {
            ui::info(format!("[DRY RUN] Would delete: {}", output_path.display()));
        }
    }
    Ok(())
}

/// Read RFC from file and validate its normalized structure.
/// Handles both legacy flat format and new `[govctl]` wire format (TOML and JSON).
pub fn read_rfc(config: &Config, path: &Path) -> Result<RfcSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read RFC: {}", path.display()))?;
    let rfc = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            let mut raw: toml::Value = toml::from_str(&content)
                .with_context(|| format!("Failed to parse RFC TOML: {}", path.display()))?;
            normalize_rfc_value(&mut raw);
            validate_toml_value(ArtifactSchema::Rfc, config, path, &raw)?;
            let wire: RfcWire = raw
                .try_into()
                .with_context(|| format!("Failed to deserialize RFC TOML: {}", path.display()))?;
            wire.into()
        }
        _ => {
            let mut raw: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse RFC JSON: {}", path.display()))?;
            normalize_rfc_json(&mut raw);
            validate_json_value(ArtifactSchema::Rfc, config, path, &raw)?;
            let wire: RfcWire = serde_json::from_value(raw)
                .with_context(|| format!("Failed to deserialize RFC JSON: {}", path.display()))?;
            wire.into()
        }
    };
    Ok(rfc)
}

/// Write RFC to file in TOML only.
/// TOML output uses the `[govctl]` wire format plus schema header.
pub fn write_rfc(
    path: &Path,
    rfc: &RfcSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<()> {
    let wire: RfcWire = rfc.clone().into();
    let body = toml::to_string_pretty(&wire)?;
    let content = with_schema_header(ArtifactSchema::Rfc, &body);
    write_file(path, &content, op, display_path)
}

/// Read clause from file and validate its normalized structure.
/// Handles both legacy flat format and new `[govctl]` + `[content]` wire format.
pub fn read_clause(config: &Config, path: &Path) -> Result<ClauseSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read clause: {}", path.display()))?;
    let clause = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            let mut raw: toml::Value = toml::from_str(&content)
                .with_context(|| format!("Failed to parse clause TOML: {}", path.display()))?;
            normalize_clause_value(&mut raw);
            validate_toml_value(ArtifactSchema::Clause, config, path, &raw)?;
            let wire: ClauseWire = raw.try_into().with_context(|| {
                format!("Failed to deserialize clause TOML: {}", path.display())
            })?;
            wire.into()
        }
        _ => {
            let mut raw: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse clause JSON: {}", path.display()))?;
            normalize_clause_json(&mut raw);
            validate_json_value(ArtifactSchema::Clause, config, path, &raw)?;
            let wire: ClauseWire = serde_json::from_value(raw).with_context(|| {
                format!("Failed to deserialize clause JSON: {}", path.display())
            })?;
            wire.into()
        }
    };
    Ok(clause)
}

/// Write clause to file in TOML only.
/// TOML output uses the `[govctl]` + `[content]` wire format plus schema header.
pub fn write_clause(
    path: &Path,
    clause: &ClauseSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<()> {
    let wire: ClauseWire = clause.clone().into();
    let body = toml::to_string_pretty(&wire)?;
    let content = with_schema_header(ArtifactSchema::Clause, &body);
    write_file(path, &content, op, display_path)
}

/// Normalize a flat RFC TOML value into the `[govctl]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_rfc_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(rfc_id) = root.remove("rfc_id") else {
        return;
    };

    let mut govctl = toml::map::Map::new();
    govctl.insert("schema".to_string(), toml::Value::Integer(1));
    govctl.insert("id".to_string(), rfc_id);
    for key in &[
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
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), toml::Value::Table(govctl));
}

/// Normalize a flat clause TOML value into the `[govctl]` + `[content]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_clause_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(clause_id) = root.remove("clause_id") else {
        return;
    };

    let mut govctl = toml::map::Map::new();
    govctl.insert("schema".to_string(), toml::Value::Integer(1));
    govctl.insert("id".to_string(), clause_id);
    for key in &[
        "title",
        "kind",
        "status",
        "anchors",
        "superseded_by",
        "since",
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), toml::Value::Table(govctl));

    let mut content = toml::map::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), toml::Value::Table(content));
}

/// Normalize a flat RFC JSON value into the `govctl` wire layout.
pub(crate) fn normalize_rfc_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(rfc_id) = root.remove("rfc_id") else {
        return;
    };
    let mut govctl = serde_json::Map::new();
    govctl.insert("schema".to_string(), serde_json::json!(1));
    govctl.insert("id".to_string(), rfc_id);
    for key in &[
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
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), serde_json::Value::Object(govctl));
}

/// Normalize a flat clause JSON value into the `govctl` + `content` wire layout.
pub(crate) fn normalize_clause_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(clause_id) = root.remove("clause_id") else {
        return;
    };
    let mut govctl = serde_json::Map::new();
    govctl.insert("schema".to_string(), serde_json::json!(1));
    govctl.insert("id".to_string(), clause_id);
    for key in &[
        "title",
        "kind",
        "status",
        "anchors",
        "superseded_by",
        "since",
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), serde_json::Value::Object(govctl));

    let mut content = serde_json::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), serde_json::Value::Object(content));
}
