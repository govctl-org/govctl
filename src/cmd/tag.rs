//! Tag management commands: new, delete, list.
//!
//! Implements controlled-vocabulary tag operations per [[RFC-0002:C-RESOURCES]].
//! Tags are stored in gov/config.toml under [tags] allowed.

use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::load_rfcs;
use crate::parse::{load_adrs, load_guards_with_warnings, load_work_items};
use anyhow::{Context, Result};
use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets::UTF8_FULL};
use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

/// Tag format regex: `^[a-z][a-z0-9-]*$` — [[RFC-0002:C-RESOURCES]]
pub static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]*$").expect("valid regex"));

fn validate_tag_format(tag: &str) -> Result<()> {
    if !TAG_RE.is_match(tag) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1101TagInvalidFormat,
            format!(
                "Invalid tag format '{tag}': tags must match ^[a-z][a-z0-9-]*$ (lowercase letters, digits, hyphens; start with a letter)"
            ),
            tag,
        )
        .into());
    }
    Ok(())
}

/// Read config.toml as a raw TOML table for in-place modification.
fn read_config_table(config: &Config) -> Result<toml::Table> {
    let config_path = config.gov_root.join("config.toml");
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
    toml::from_str::<toml::Table>(&content)
        .with_context(|| format!("Failed to parse config: {}", config_path.display()))
}

/// Write a modified TOML table back to config.toml.
fn write_config_table(config: &Config, table: &toml::Table) -> Result<()> {
    let config_path = config.gov_root.join("config.toml");
    let content = toml::to_string_pretty(table).with_context(|| "Failed to serialize config")?;
    std::fs::write(&config_path, content)
        .with_context(|| format!("Failed to write config: {}", config_path.display()))?;
    Ok(())
}

/// Get the current allowed tags array from a TOML table.
fn get_allowed_tags(table: &toml::Table) -> Result<Vec<String>> {
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
fn set_allowed_tags(table: &mut toml::Table, tags: Vec<String>) -> Result<()> {
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

/// Count how many artifacts use a given tag across all artifact types.
fn count_tag_usage(config: &Config, tag: &str) -> Result<usize> {
    let mut count = 0;

    let rfcs = load_rfcs(config).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    for rfc_index in &rfcs {
        if rfc_index.rfc.tags.iter().any(|t| t == tag) {
            count += 1;
        }
        for clause in &rfc_index.clauses {
            if clause.spec.tags.iter().any(|t| t == tag) {
                count += 1;
            }
        }
    }

    let adrs = load_adrs(config).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    for adr in &adrs {
        if adr.spec.govctl.tags.iter().any(|t| t == tag) {
            count += 1;
        }
    }

    let items = load_work_items(config).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    for item in &items {
        if item.spec.govctl.tags.iter().any(|t| t == tag) {
            count += 1;
        }
    }

    let guard_result = load_guards_with_warnings(config).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    for guard in &guard_result.items {
        if guard.spec.govctl.tags.iter().any(|t| t == tag) {
            count += 1;
        }
    }

    Ok(count)
}

/// Add a new allowed tag to config.toml [tags] allowed.
pub fn tag_new(config: &Config, tag: &str, op: crate::write::WriteOp) -> Result<Vec<Diagnostic>> {
    validate_tag_format(tag)?;

    let mut table = read_config_table(config)?;
    let mut allowed = get_allowed_tags(&table)?;

    if allowed.contains(&tag.to_string()) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1102TagAlreadyExists,
            format!("Tag '{tag}' already exists in [tags] allowed"),
            tag,
        )
        .into());
    }

    allowed.push(tag.to_string());
    set_allowed_tags(&mut table, allowed)?;

    if !op.is_preview() {
        write_config_table(config, &table)?;
        println!("Added tag: {tag}");
    } else {
        println!("Would add tag: {tag}");
    }
    Ok(vec![])
}

/// Remove an allowed tag from config.toml [tags] allowed.
/// Fails if any artifact still references the tag.
pub fn tag_delete(
    config: &Config,
    tag: &str,
    op: crate::write::WriteOp,
) -> Result<Vec<Diagnostic>> {
    let mut table = read_config_table(config)?;
    let allowed = get_allowed_tags(&table)?;

    if !allowed.contains(&tag.to_string()) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1103TagNotFound,
            format!("Tag '{tag}' not found in [tags] allowed"),
            tag,
        )
        .into());
    }

    // Check for usage across all artifact types — [[RFC-0002:C-RESOURCES]]
    let usage = count_tag_usage(config, tag)?;
    if usage > 0 {
        return Err(Diagnostic::new(
            DiagnosticCode::E1104TagStillReferenced,
            format!(
                "Cannot delete tag '{tag}': still used by {usage} artifact(s). Remove the tag from all artifacts first."
            ),
            tag,
        )
        .into());
    }

    let new_allowed: Vec<String> = allowed.into_iter().filter(|t| t != tag).collect();
    set_allowed_tags(&mut table, new_allowed)?;

    if !op.is_preview() {
        write_config_table(config, &table)?;
        println!("Deleted tag: {tag}");
    } else {
        println!("Would delete tag: {tag}");
    }
    Ok(vec![])
}

/// List all allowed tags and their usage counts across all artifacts.
pub fn tag_list(config: &Config, output: OutputFormat) -> Result<Vec<Diagnostic>> {
    let table = read_config_table(config)?;
    let allowed = get_allowed_tags(&table)?;

    #[derive(Serialize)]
    struct TagEntry {
        tag: String,
        usage: usize,
    }

    let entries: Vec<TagEntry> = allowed
        .iter()
        .map(|tag| {
            let usage = count_tag_usage(config, tag).unwrap_or(0);
            TagEntry {
                tag: tag.clone(),
                usage,
            }
        })
        .collect();

    match output {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".to_string())
            );
        }
        OutputFormat::Plain => {
            for e in &entries {
                println!("{}\t{}", e.tag, e.usage);
            }
        }
        OutputFormat::Table => {
            let mut table_out = Table::new();
            table_out
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("Tag").add_attribute(Attribute::Bold),
                    Cell::new("Usage").add_attribute(Attribute::Bold),
                ]);
            for e in &entries {
                table_out.add_row(vec![Cell::new(&e.tag), Cell::new(e.usage.to_string())]);
            }
            println!("{table_out}");
        }
    }

    Ok(vec![])
}
