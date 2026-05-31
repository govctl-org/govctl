use super::ops::FileOp;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ReleasesFile;
use crate::schema::{ArtifactSchema, validate_toml_value, with_schema_header};
use std::fs;

pub(super) fn plan_release_upgrade(config: &Config) -> anyhow::Result<Option<Vec<FileOp>>> {
    let path = config.releases_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)?;
    let mut raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid releases.toml: {e}"),
            config.display_path(&path).display().to_string(),
        )
    })?;

    let needs_upgrade = {
        let table = raw.as_table();
        let govctl = table
            .and_then(|t| t.get("govctl"))
            .and_then(toml::Value::as_table);
        let has_schema = govctl
            .and_then(|g| g.get("schema"))
            .and_then(toml::Value::as_integer)
            == Some(1);
        !has_schema
    };

    if !needs_upgrade {
        return Ok(None);
    }

    // Normalize: ensure [govctl] schema = 1
    if let Some(root) = raw.as_table_mut() {
        let govctl = root
            .entry("govctl".to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        if let Some(table) = govctl.as_table_mut() {
            table
                .entry("schema".to_string())
                .or_insert(toml::Value::Integer(1));
        }
    }

    validate_toml_value(ArtifactSchema::Release, config, &path, &raw)?;
    let releases: ReleasesFile = raw.try_into().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid releases structure: {e}"),
            config.display_path(&path).display().to_string(),
        )
    })?;
    let body = toml::to_string_pretty(&releases)?;
    Ok(Some(vec![FileOp::Write {
        path,
        content: with_schema_header(ArtifactSchema::Release, &body),
    }]))
}
