//! Tag management commands: new, delete, list.
//!
//! Implements controlled-vocabulary tag operations per [[RFC-0002:C-RESOURCES]].
//! Tags are stored in gov/config.toml under [tags] allowed.

mod registry;

use crate::OutputFormat;
use crate::cmd::output::{print_json_array, table_with_bold_headers};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::load_rfcs;
use crate::parse::{load_adrs, load_guards_with_warnings, load_work_items};
use comfy_table::Cell;
use registry::{
    get_allowed_tags, has_allowed_tag, read_config_table, set_allowed_tags, validate_tag_format,
    write_config_table,
};
pub(crate) use registry::{validate_artifact_tag, validate_registered_tag};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct TagEntry {
    tag: String,
    usage: usize,
}

/// Add a new allowed tag to config.toml [tags] allowed.
pub fn tag_new(
    config: &Config,
    tag: &str,
    op: crate::write::WriteOp,
) -> DiagnosticResult<Diagnostics> {
    validate_tag_format(tag)?;

    let mut table = read_config_table(config)?;
    let mut allowed = get_allowed_tags(&table)?;

    if has_allowed_tag(&allowed, tag) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1102TagAlreadyExists,
            format!("Tag '{tag}' already exists in [tags] allowed"),
            tag,
        ));
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
) -> DiagnosticResult<Diagnostics> {
    let mut table = read_config_table(config)?;
    let allowed = get_allowed_tags(&table)?;

    if !has_allowed_tag(&allowed, tag) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1103TagNotFound,
            format!("Tag '{tag}' not found in [tags] allowed"),
            tag,
        ));
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
        ));
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
pub fn tag_list(config: &Config, output: OutputFormat) -> DiagnosticResult<Diagnostics> {
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

fn print_tag_entries(entries: &[TagEntry], output: OutputFormat) {
    match output {
        OutputFormat::Json => {
            print_json_array(entries);
        }
        OutputFormat::Plain => {
            for entry in entries {
                println!("{}\t{}", entry.tag, entry.usage);
            }
        }
        OutputFormat::Table => {
            let mut table = table_with_bold_headers(&["Tag", "Usage"]);
            for entry in entries {
                table.add_row(vec![
                    Cell::new(&entry.tag),
                    Cell::new(entry.usage.to_string()),
                ]);
            }
            println!("{table}");
        }
    }
}

fn build_tag_usage_map(config: &Config) -> DiagnosticResult<HashMap<String, usize>> {
    let mut usage: HashMap<String, usize> = HashMap::new();

    let rfcs = load_rfcs(config).map_err(Diagnostic::from)?;
    for rfc_index in &rfcs {
        increment_tag_usage(&mut usage, &rfc_index.rfc.tags);
        for clause in &rfc_index.clauses {
            increment_tag_usage(&mut usage, &clause.spec.tags);
        }
    }

    let adrs = load_adrs(config)?;
    for adr in &adrs {
        increment_tag_usage(&mut usage, &adr.spec.govctl.tags);
    }

    let items = load_work_items(config)?;
    for item in &items {
        increment_tag_usage(&mut usage, &item.spec.govctl.tags);
    }

    let guard_result = load_guards_with_warnings(config)?;
    for guard in &guard_result.items {
        increment_tag_usage(&mut usage, &guard.spec.govctl.tags);
    }

    Ok(usage)
}

fn increment_tag_usage(map: &mut HashMap<String, usize>, tags: &[String]) {
    for tag in tags {
        *map.entry(tag.clone()).or_insert(0) += 1;
    }
}
