//! Tag management commands: new, delete, list.
//!
//! Implements controlled-vocabulary tag operations per [[RFC-0002:C-RESOURCES]].
//! Tags are stored in gov/config.toml under [tags] allowed.

mod output;
mod registry;
mod usage;

use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use anyhow::Result;
use output::{TagEntry, print_tag_entries};
pub use registry::tag_re;
use registry::{
    get_allowed_tags, read_config_table, set_allowed_tags, validate_tag_format, write_config_table,
};
use usage::build_tag_usage_map;

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
    let usage_map = build_tag_usage_map(config)?;
    let usage = usage_map.get(tag).copied().unwrap_or(0);
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

    let usage_map = build_tag_usage_map(config)?;
    let entries: Vec<TagEntry> = allowed
        .iter()
        .map(|tag| {
            let usage = usage_map.get(tag).copied().unwrap_or(0);
            TagEntry {
                tag: tag.clone(),
                usage,
            }
        })
        .collect();

    print_tag_entries(&entries, output);
    Ok(vec![])
}
