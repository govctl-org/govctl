use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use regex::Regex;
use std::sync::LazyLock;

/// Tag format regex: `^[a-z][a-z0-9-]*$` — [[RFC-0002:C-RESOURCES]]
static TAG_RE_RESULT: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]*$"));

/// Return a reference to the compiled tag format regex.
pub fn tag_re() -> Result<&'static Regex, regex::Error> {
    TAG_RE_RESULT.as_ref().map_err(|e| e.clone())
}

pub(super) fn validate_tag_format(tag: &str) -> DiagnosticResult<()> {
    let re = tag_re().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0806InvalidPattern,
            format!("Failed to compile tag regex: {e}"),
            "",
        )
    })?;
    if !re.is_match(tag) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1101TagInvalidFormat,
            format!(
                "Invalid tag format '{tag}': tags must match ^[a-z][a-z0-9-]*$ (lowercase letters, digits, hyphens; start with a letter)"
            ),
            tag,
        ));
    }
    Ok(())
}

/// Read config.toml as a raw TOML table for in-place modification.
pub(super) fn read_config_table(config: &Config) -> DiagnosticResult<toml::Table> {
    let config_path = config.gov_root.join("config.toml");
    let content = std::fs::read_to_string(&config_path).map_err(|err| {
        Diagnostic::io_error("read config", err, config_path.display().to_string())
    })?;
    toml::from_str::<toml::Table>(&content).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!("Failed to parse config: {err}"),
            config_path.display().to_string(),
        )
    })
}

/// Write a modified TOML table back to config.toml.
pub(super) fn write_config_table(config: &Config, table: &toml::Table) -> DiagnosticResult<()> {
    let config_path = config.gov_root.join("config.toml");
    let content = toml::to_string_pretty(table).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!("Failed to serialize config: {err}"),
            config_path.display().to_string(),
        )
    })?;
    std::fs::write(&config_path, content).map_err(|err| {
        Diagnostic::io_error("write config", err, config_path.display().to_string())
    })?;
    Ok(())
}

/// Get the current allowed tags array from a TOML table.
pub(super) fn get_allowed_tags(table: &toml::Table) -> DiagnosticResult<Vec<String>> {
    let Some(tags_val) = table.get("tags") else {
        return Ok(vec![]);
    };
    let tags_table = tags_val.as_table().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            "'tags' in config.toml must be a table",
            "gov/config.toml",
        )
    })?;
    let Some(allowed_val) = tags_table.get("allowed") else {
        return Ok(vec![]);
    };
    let arr = allowed_val.as_array().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            "'tags.allowed' in config.toml must be an array",
            "gov/config.toml",
        )
    })?;
    let mut tags = Vec::new();
    for item in arr {
        let s = item.as_str().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                "'tags.allowed' items must be strings",
                "gov/config.toml",
            )
        })?;
        tags.push(s.to_string());
    }
    Ok(tags)
}

/// Set the allowed tags array in a TOML table.
pub(super) fn set_allowed_tags(table: &mut toml::Table, tags: Vec<String>) -> DiagnosticResult<()> {
    let arr: toml::value::Array = tags.into_iter().map(toml::Value::String).collect();

    let tags_table = table
        .entry("tags")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                "'tags' in config.toml must be a table",
                "gov/config.toml",
            )
        })?;

    tags_table.insert("allowed".to_string(), toml::Value::Array(arr));
    Ok(())
}
