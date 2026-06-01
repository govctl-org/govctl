use super::LoadResult;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::schema::{ArtifactSchema, validate_toml_value, with_schema_header};
use crate::ui;
use crate::write::WriteOp;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;

pub(super) fn load_toml_dir<T>(
    dir: &Path,
    load_one: impl Fn(&Path) -> Result<T, Diagnostic>,
    sort_items: impl Fn(&mut Vec<T>),
) -> Result<LoadResult<T>, Diagnostic> {
    if !dir.exists() {
        return Ok(LoadResult {
            items: vec![],
            warnings: vec![],
        });
    }

    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| Diagnostic::io_error("read TOML directory", e, dir.display().to_string()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load_one(&path) {
                Ok(item) => items.push(item),
                Err(e) => warnings.push(e),
            }
        }
    }

    sort_items(&mut items);
    Ok(LoadResult { items, warnings })
}

pub(super) fn load_toml_spec<T>(
    config: &Config,
    path: &Path,
    schema: ArtifactSchema,
    diagnostic_code: DiagnosticCode,
    invalid_toml_context: &str,
    invalid_structure_context: &str,
    prepare_schema_value: impl FnOnce(&mut toml::Value),
) -> Result<T, Diagnostic>
where
    T: DeserializeOwned,
{
    let content = std::fs::read_to_string(path)
        .map_err(|e| Diagnostic::io_error("read TOML file", e, path.display().to_string()))?;

    let raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            diagnostic_code,
            format!("{invalid_toml_context}: {e}"),
            path.display().to_string(),
        )
    })?;
    let mut schema_raw = raw.clone();
    prepare_schema_value(&mut schema_raw);
    validate_toml_value(schema, config, path, &schema_raw)?;
    raw.try_into().map_err(|e| {
        Diagnostic::new(
            diagnostic_code,
            format!("{invalid_structure_context}: {e}"),
            path.display().to_string(),
        )
    })
}

pub(super) fn write_toml_spec<T>(
    path: &Path,
    schema: ArtifactSchema,
    spec: &T,
    op: WriteOp,
    display_path: Option<&Path>,
    serialize_context: &str,
) -> Result<(), Diagnostic>
where
    T: Serialize,
{
    let diagnostic_path = display_path.unwrap_or(path);
    let body = toml::to_string_pretty(spec).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("{serialize_context}: {e}"),
            diagnostic_path.display().to_string(),
        )
    })?;
    let content = with_schema_header(schema, &body);

    match op {
        WriteOp::Execute => {
            std::fs::write(path, &content).map_err(|e| {
                Diagnostic::io_error("write TOML file", e, diagnostic_path.display().to_string())
            })?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(diagnostic_path, &content);
        }
    }

    Ok(())
}
