//! Edit command implementation - modify artifacts.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::{find_clause_json, find_rfc_json};
use crate::parse::{load_adrs, load_work_items, write_adr, write_work_item};
use crate::ui;
use crate::write::{
    WriteOp, read_clause, read_rfc, update_clause_field, update_rfc_field, write_clause, write_rfc,
};
use anyhow::Context;
use regex::Regex;
use std::io::Read;
use std::path::Path;

/// Options for matching array elements (per ADR-0007)
#[derive(Debug, Clone, Default)]
pub struct MatchOptions<'a> {
    /// Pattern to match (substring by default)
    pub pattern: Option<&'a str>,
    /// Match by index (0-based, negative from end)
    pub at: Option<i32>,
    /// Exact match (case-sensitive)
    pub exact: bool,
    /// Regex pattern
    pub regex: bool,
    /// Allow removing all matches
    pub all: bool,
}

/// Result of matching against an array
#[derive(Debug)]
pub enum MatchResult {
    /// No matches found
    None,
    /// Single match at index
    Single(usize),
    /// Multiple matches at indices
    Multiple(Vec<usize>),
}

/// Find matching indices in a list of strings
fn find_matches(items: &[&str], opts: &MatchOptions) -> anyhow::Result<MatchResult> {
    // Index-based matching
    if let Some(idx) = opts.at {
        let len = items.len() as i32;
        let actual_idx = if idx < 0 { len + idx } else { idx };
        if actual_idx < 0 || actual_idx >= len {
            anyhow::bail!(
                "Index {} out of range (array has {} items)",
                idx,
                items.len()
            );
        }
        return Ok(MatchResult::Single(actual_idx as usize));
    }

    // Pattern-based matching
    let pattern = opts
        .pattern
        .ok_or_else(|| anyhow::anyhow!("No pattern or index provided"))?;

    let matches: Vec<usize> = if opts.regex {
        let re = Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
        items
            .iter()
            .enumerate()
            .filter(|(_, s)| re.is_match(s))
            .map(|(i, _)| i)
            .collect()
    } else if opts.exact {
        items
            .iter()
            .enumerate()
            .filter(|(_, s)| **s == pattern)
            .map(|(i, _)| i)
            .collect()
    } else {
        // Default: case-insensitive substring
        let pattern_lower = pattern.to_lowercase();
        items
            .iter()
            .enumerate()
            .filter(|(_, s)| s.to_lowercase().contains(&pattern_lower))
            .map(|(i, _)| i)
            .collect()
    };

    Ok(match matches.len() {
        0 => MatchResult::None,
        1 => MatchResult::Single(matches[0]),
        _ => MatchResult::Multiple(matches),
    })
}

/// Format error message for multiple matches
fn format_multiple_match_error(
    id: &str,
    field: &str,
    pattern: &str,
    items: &[&str],
    indices: &[usize],
) -> String {
    let mut msg = format!(
        "{} items match '{}' in {}.{}:\n",
        indices.len(),
        pattern,
        id,
        field
    );
    for &i in indices {
        msg.push_str(&format!("  [{}] {}\n", i, items[i]));
    }
    msg.push_str("\nOptions:\n");
    msg.push_str(
        "  • Use more specific pattern\n  • Use --at <index> to select one\n  • Use --all to remove all matches"
    );
    msg
}

/// Read value from stdin, trimming trailing newline
fn read_stdin() -> anyhow::Result<String> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read from stdin")?;
    // Trim trailing newline that HEREDOC adds
    Ok(buffer.trim_end_matches('\n').to_string())
}

/// Resolve value from either argument or stdin
fn resolve_value(value: Option<&str>, stdin: bool) -> anyhow::Result<String> {
    match (value, stdin) {
        (Some(v), false) => Ok(v.to_string()),
        (None, true) => read_stdin(),
        (None, false) => anyhow::bail!("Provide a value or use --stdin"),
        (Some(_), true) => anyhow::bail!("Cannot use both value and --stdin"),
    }
}

/// Edit clause text
pub fn edit_clause(
    config: &Config,
    clause_id: &str,
    text: Option<&str>,
    text_file: Option<&Path>,
    stdin: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let clause_path = find_clause_json(config, clause_id)
        .ok_or_else(|| anyhow::anyhow!("Clause not found: {clause_id}"))?;

    let mut clause = read_clause(&clause_path)?;

    let new_text = match (text, text_file, stdin) {
        (Some(t), None, false) => t.to_string(),
        (None, Some(path), false) => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read text file: {}", path.display()))?,
        (None, None, true) => read_stdin()?,
        (None, None, false) => anyhow::bail!("Provide --text, --text-file, or --stdin"),
        _ => unreachable!("clap arg group ensures mutual exclusivity"),
    };

    clause.text = new_text;
    write_clause(&clause_path, &clause, op)?;

    if !op.is_preview() {
        ui::updated("clause", clause_id);
    }
    Ok(vec![])
}

/// Set a field value
pub fn set_field(
    config: &Config,
    id: &str,
    field: &str,
    value: Option<&str>,
    stdin: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let value = resolve_value(value, stdin)?;
    let value = value.as_str();
    // Determine if it's an RFC, clause, ADR, or Work Item
    if id.contains(':') {
        // It's a clause (RFC-0001:C-NAME)
        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        // Validate field value
        crate::validate::validate_field(
            config,
            id,
            crate::validate::ArtifactKind::Clause,
            field,
            value,
        )?;

        let mut clause = read_clause(&clause_path)?;
        update_clause_field(&mut clause, field, value)?;
        write_clause(&clause_path, &clause, op)?;

        if !op.is_preview() {
            ui::field_set(id, field, value);
        }
    } else if id.starts_with("RFC-") {
        // It's an RFC
        let rfc_path =
            find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

        // Validate field value
        crate::validate::validate_field(
            config,
            id,
            crate::validate::ArtifactKind::Rfc,
            field,
            value,
        )?;

        let mut rfc = read_rfc(&rfc_path)?;
        update_rfc_field(&mut rfc, field, value)?;
        write_rfc(&rfc_path, &rfc, op)?;

        if !op.is_preview() {
            ui::field_set(id, field, value);
        }
    } else if id.starts_with("ADR-") {
        // It's an ADR - load, modify, write TOML
        let mut entry = load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        match field {
            "status" | "govctl.status" => {
                entry.spec.govctl.status = serde_json::from_str(&format!("\"{value}\""))
                    .map_err(|_| anyhow::anyhow!("Invalid status: {value}"))?;
            }
            "title" | "govctl.title" => entry.spec.govctl.title = value.to_string(),
            "date" | "govctl.date" => entry.spec.govctl.date = value.to_string(),
            "context" | "content.context" => entry.spec.content.context = value.to_string(),
            "decision" | "content.decision" => entry.spec.content.decision = value.to_string(),
            "consequences" | "content.consequences" => {
                entry.spec.content.consequences = value.to_string()
            }
            _ => anyhow::bail!("Unknown ADR field: {field}"),
        }

        write_adr(&entry.path, &entry.spec, op)?;
        if !op.is_preview() {
            ui::field_set(id, field, value);
        }
    } else if id.starts_with("WI-")
        || (id.contains("-") && !id.starts_with("RFC-") && !id.starts_with("ADR-"))
    {
        // It's a work item
        let mut entry = load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))?;

        match field {
            "status" | "govctl.status" => {
                entry.spec.govctl.status = serde_json::from_str(&format!("\"{value}\""))
                    .map_err(|_| anyhow::anyhow!("Invalid status: {value}"))?;
            }
            "title" | "govctl.title" => entry.spec.govctl.title = value.to_string(),
            "description" | "content.description" => {
                entry.spec.content.description = value.to_string()
            }
            "notes" | "content.notes" => entry.spec.content.notes = value.to_string(),
            _ => anyhow::bail!("Unknown work item field: {field}"),
        }

        write_work_item(&entry.path, &entry.spec, op)?;
        if !op.is_preview() {
            ui::field_set(id, field, value);
        }
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}

/// Get a field value
pub fn get_field(
    config: &Config,
    id: &str,
    field: Option<&str>,
) -> anyhow::Result<Vec<Diagnostic>> {
    if id.contains(':') {
        // It's a clause
        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        let clause = read_clause(&clause_path)?;

        if let Some(f) = field {
            let value = match f {
                "text" => clause.text,
                "title" => clause.title,
                "status" => clause.status.as_ref().to_string(),
                "kind" => clause.kind.as_ref().to_string(),
                "superseded_by" => clause.superseded_by.unwrap_or_default(),
                "since" => clause.since.unwrap_or_default(),
                _ => anyhow::bail!("Unknown field: {f}"),
            };
            println!("{value}");
        } else {
            println!("{}", serde_json::to_string_pretty(&clause)?);
        }
    } else if id.starts_with("RFC-") {
        let rfc_path =
            find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

        let rfc = read_rfc(&rfc_path)?;

        if let Some(f) = field {
            let value = match f {
                "status" => rfc.status.as_ref().to_string(),
                "phase" => rfc.phase.as_ref().to_string(),
                "title" => rfc.title,
                "version" => rfc.version,
                "owners" => rfc.owners.join(", "),
                "created" => rfc.created,
                "updated" => rfc.updated.unwrap_or_default(),
                _ => anyhow::bail!("Unknown field: {f}"),
            };
            println!("{value}");
        } else {
            println!("{}", serde_json::to_string_pretty(&rfc)?);
        }
    } else if id.starts_with("ADR-") {
        let entry = load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        if let Some(f) = field {
            let value = match f {
                "status" => entry.spec.govctl.status.as_ref().to_string(),
                "title" => entry.spec.govctl.title,
                "date" => entry.spec.govctl.date,
                "superseded_by" => entry.spec.govctl.superseded_by.unwrap_or_default(),
                "refs" => entry.spec.govctl.refs.join(", "),
                "context" => entry.spec.content.context,
                "decision" => entry.spec.content.decision,
                "consequences" => entry.spec.content.consequences,
                "alternatives" => entry
                    .spec
                    .content
                    .alternatives
                    .iter()
                    .map(|a| format!("[{}] {}", a.status.as_ref(), a.text))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => anyhow::bail!("Unknown field: {f}"),
            };
            println!("{value}");
        } else {
            println!("{}", toml::to_string_pretty(&entry.spec)?);
        }
    } else if id.starts_with("WI-")
        || (id.contains("-") && !id.starts_with("RFC-") && !id.starts_with("ADR-"))
    {
        // It's a work item
        let entry = load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))?;

        if let Some(f) = field {
            let value = match f {
                "status" => entry.spec.govctl.status.as_ref().to_string(),
                "title" => entry.spec.govctl.title,
                "started" => entry.spec.govctl.started.unwrap_or_default(),
                "completed" => entry.spec.govctl.completed.unwrap_or_default(),
                "refs" => entry.spec.govctl.refs.join(", "),
                "description" => entry.spec.content.description,
                "acceptance_criteria" => entry
                    .spec
                    .content
                    .acceptance_criteria
                    .iter()
                    .map(|c| format!("[{}] {}", c.status.as_ref(), c.text))
                    .collect::<Vec<_>>()
                    .join("\n"),
                "decisions" => entry
                    .spec
                    .content
                    .decisions
                    .iter()
                    .map(|d| format!("[{}] {}", d.status.as_ref(), d.text))
                    .collect::<Vec<_>>()
                    .join("\n"),
                "notes" => entry.spec.content.notes,
                _ => anyhow::bail!("Unknown field: {f}"),
            };
            println!("{value}");
        } else {
            println!("{}", toml::to_string_pretty(&entry.spec)?);
        }
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}

/// Add a value to an array field
pub fn add_to_field(
    config: &Config,
    id: &str,
    field: &str,
    value: Option<&str>,
    stdin: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let value = resolve_value(value, stdin)?;
    let value = value.as_str();

    if id.starts_with("RFC-") && !id.contains(':') {
        // RFC array fields: owners
        let rfc_path =
            find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

        let mut rfc = read_rfc(&rfc_path)?;

        match field {
            "owners" => {
                if !rfc.owners.contains(&value.to_string()) {
                    rfc.owners.push(value.to_string());
                }
            }
            _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
        }

        write_rfc(&rfc_path, &rfc, op)?;
        if !op.is_preview() {
            ui::field_added(id, field, value);
        }
    } else if id.contains(':') {
        // Clause array fields: anchors
        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        let mut clause = read_clause(&clause_path)?;

        match field {
            "anchors" => {
                if !clause.anchors.contains(&value.to_string()) {
                    clause.anchors.push(value.to_string());
                }
            }
            _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
        }

        write_clause(&clause_path, &clause, op)?;
        if !op.is_preview() {
            ui::field_added(id, field, value);
        }
    } else if id.starts_with("ADR-") {
        let mut entry = load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        match field {
            "refs" => {
                if !entry.spec.govctl.refs.contains(&value.to_string()) {
                    entry.spec.govctl.refs.push(value.to_string());
                }
            }
            "alternatives" => {
                use crate::model::Alternative;
                if !entry
                    .spec
                    .content
                    .alternatives
                    .iter()
                    .any(|a| a.text == value)
                {
                    entry
                        .spec
                        .content
                        .alternatives
                        .push(Alternative::new(value));
                }
            }
            _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
        }

        write_adr(&entry.path, &entry.spec, op)?;
        if !op.is_preview() {
            ui::field_added(id, field, value);
        }
    } else if id.starts_with("WI-") || id.contains("-") {
        let mut entry = load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))?;

        match field {
            "refs" => {
                if !entry.spec.govctl.refs.contains(&value.to_string()) {
                    entry.spec.govctl.refs.push(value.to_string());
                }
            }
            "acceptance_criteria" => {
                use crate::model::ChecklistItem;
                if !entry
                    .spec
                    .content
                    .acceptance_criteria
                    .iter()
                    .any(|c| c.text == value)
                {
                    entry
                        .spec
                        .content
                        .acceptance_criteria
                        .push(ChecklistItem::new(value));
                }
            }
            "decisions" => {
                use crate::model::ChecklistItem;
                if !entry.spec.content.decisions.iter().any(|d| d.text == value) {
                    entry.spec.content.decisions.push(ChecklistItem::new(value));
                }
            }
            _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
        }

        write_work_item(&entry.path, &entry.spec, op)?;
        if !op.is_preview() {
            ui::field_added(id, field, value);
        }
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}

/// Remove matching items from an array field (per ADR-0007)
pub fn remove_from_field(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    if id.starts_with("RFC-") && !id.contains(':') {
        let rfc_path =
            find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

        let mut rfc = read_rfc(&rfc_path)?;

        match field {
            "owners" => {
                let items: Vec<&str> = rfc.owners.iter().map(|s| s.as_str()).collect();
                let to_remove = resolve_matches(id, field, &items, opts)?;
                let removed: Vec<_> = to_remove.iter().map(|&i| rfc.owners[i].clone()).collect();
                remove_indices(&mut rfc.owners, &to_remove);

                write_rfc(&rfc_path, &rfc, op)?;
                if !op.is_preview() {
                    for item in &removed {
                        ui::field_removed(id, field, item);
                    }
                }
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }
    } else if id.contains(':') {
        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        let mut clause = read_clause(&clause_path)?;

        match field {
            "anchors" => {
                let items: Vec<&str> = clause.anchors.iter().map(|s| s.as_str()).collect();
                let to_remove = resolve_matches(id, field, &items, opts)?;
                let removed: Vec<_> = to_remove
                    .iter()
                    .map(|&i| clause.anchors[i].clone())
                    .collect();
                remove_indices(&mut clause.anchors, &to_remove);

                write_clause(&clause_path, &clause, op)?;
                if !op.is_preview() {
                    for item in &removed {
                        ui::field_removed(id, field, item);
                    }
                }
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }
    } else if id.starts_with("ADR-") {
        let mut entry = load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        match field {
            "refs" => {
                let items: Vec<&str> = entry.spec.govctl.refs.iter().map(|s| s.as_str()).collect();
                let to_remove = resolve_matches(id, field, &items, opts)?;
                let removed: Vec<_> = to_remove
                    .iter()
                    .map(|&i| entry.spec.govctl.refs[i].clone())
                    .collect();
                remove_indices(&mut entry.spec.govctl.refs, &to_remove);

                write_adr(&entry.path, &entry.spec, op)?;
                if !op.is_preview() {
                    for item in &removed {
                        ui::field_removed(id, field, item);
                    }
                }
            }
            "alternatives" => {
                let items: Vec<&str> = entry
                    .spec
                    .content
                    .alternatives
                    .iter()
                    .map(|a| a.text.as_str())
                    .collect();
                let to_remove = resolve_matches(id, field, &items, opts)?;
                let removed: Vec<_> = to_remove
                    .iter()
                    .map(|&i| entry.spec.content.alternatives[i].text.clone())
                    .collect();
                remove_indices(&mut entry.spec.content.alternatives, &to_remove);

                write_adr(&entry.path, &entry.spec, op)?;
                if !op.is_preview() {
                    for item in &removed {
                        ui::field_removed(id, field, item);
                    }
                }
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }
    } else if id.starts_with("WI-") || id.contains("-") {
        let mut entry = load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))?;

        match field {
            "refs" => {
                let items: Vec<&str> = entry.spec.govctl.refs.iter().map(|s| s.as_str()).collect();
                let to_remove = resolve_matches(id, field, &items, opts)?;
                let removed: Vec<_> = to_remove
                    .iter()
                    .map(|&i| entry.spec.govctl.refs[i].clone())
                    .collect();
                remove_indices(&mut entry.spec.govctl.refs, &to_remove);

                write_work_item(&entry.path, &entry.spec, op)?;
                if !op.is_preview() {
                    for item in &removed {
                        ui::field_removed(id, field, item);
                    }
                }
            }
            "acceptance_criteria" => {
                let items: Vec<&str> = entry
                    .spec
                    .content
                    .acceptance_criteria
                    .iter()
                    .map(|c| c.text.as_str())
                    .collect();
                let to_remove = resolve_matches(id, field, &items, opts)?;
                let removed: Vec<_> = to_remove
                    .iter()
                    .map(|&i| entry.spec.content.acceptance_criteria[i].text.clone())
                    .collect();
                remove_indices(&mut entry.spec.content.acceptance_criteria, &to_remove);

                write_work_item(&entry.path, &entry.spec, op)?;
                if !op.is_preview() {
                    for item in &removed {
                        ui::field_removed(id, field, item);
                    }
                }
            }
            "decisions" => {
                let items: Vec<&str> = entry
                    .spec
                    .content
                    .decisions
                    .iter()
                    .map(|d| d.text.as_str())
                    .collect();
                let to_remove = resolve_matches(id, field, &items, opts)?;
                let removed: Vec<_> = to_remove
                    .iter()
                    .map(|&i| entry.spec.content.decisions[i].text.clone())
                    .collect();
                remove_indices(&mut entry.spec.content.decisions, &to_remove);

                write_work_item(&entry.path, &entry.spec, op)?;
                if !op.is_preview() {
                    for item in &removed {
                        ui::field_removed(id, field, item);
                    }
                }
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}

/// Resolve match result to list of indices, handling errors for no/multiple matches
fn resolve_matches(
    id: &str,
    field: &str,
    items: &[&str],
    opts: &MatchOptions,
) -> anyhow::Result<Vec<usize>> {
    if items.is_empty() {
        anyhow::bail!("Field {}.{} is empty", id, field);
    }

    match find_matches(items, opts)? {
        MatchResult::None => {
            let pattern = opts.pattern.unwrap_or("<index>");
            anyhow::bail!("No items match '{}' in {}.{}", pattern, id, field);
        }
        MatchResult::Single(idx) => Ok(vec![idx]),
        MatchResult::Multiple(indices) => {
            if opts.all {
                Ok(indices)
            } else {
                let pattern = opts.pattern.unwrap_or("");
                anyhow::bail!(
                    "{}",
                    format_multiple_match_error(id, field, pattern, items, &indices)
                );
            }
        }
    }
}

/// Remove items at given indices (must be sorted descending to avoid index shift)
fn remove_indices<T>(vec: &mut Vec<T>, indices: &[usize]) {
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_by(|a, b| b.cmp(a)); // descending
    for i in sorted {
        vec.remove(i);
    }
}

/// Mark a checklist item with a new status (per ADR-0007)
pub fn tick_item(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    status: crate::TickStatus,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    use crate::model::{AlternativeStatus, ChecklistStatus};

    if id.starts_with("WI-")
        || (id.contains("-") && !id.starts_with("RFC-") && !id.starts_with("ADR-"))
    {
        let mut entry = load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))?;

        let new_status = match status {
            crate::TickStatus::Done => ChecklistStatus::Done,
            crate::TickStatus::Pending => ChecklistStatus::Pending,
            crate::TickStatus::Cancelled => ChecklistStatus::Cancelled,
        };

        let ticked_text = match field {
            "acceptance_criteria" => {
                let items: Vec<&str> = entry
                    .spec
                    .content
                    .acceptance_criteria
                    .iter()
                    .map(|c| c.text.as_str())
                    .collect();
                let indices = resolve_single_match(id, field, &items, opts)?;
                let text = entry.spec.content.acceptance_criteria[indices].text.clone();
                entry.spec.content.acceptance_criteria[indices].status = new_status;
                text
            }
            "decisions" => {
                let items: Vec<&str> = entry
                    .spec
                    .content
                    .decisions
                    .iter()
                    .map(|d| d.text.as_str())
                    .collect();
                let indices = resolve_single_match(id, field, &items, opts)?;
                let text = entry.spec.content.decisions[indices].text.clone();
                entry.spec.content.decisions[indices].status = new_status;
                text
            }
            _ => anyhow::bail!("Unknown field for tick: {field}"),
        };

        write_work_item(&entry.path, &entry.spec, op)?;
        if !op.is_preview() {
            ui::ticked(&ticked_text, new_status.as_ref());
        }
    } else if id.starts_with("ADR-") {
        let mut entry = load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        let new_status = match status {
            crate::TickStatus::Done => AlternativeStatus::Accepted,
            crate::TickStatus::Pending => AlternativeStatus::Considered,
            crate::TickStatus::Cancelled => AlternativeStatus::Rejected,
        };

        let ticked_text = match field {
            "alternatives" => {
                let items: Vec<&str> = entry
                    .spec
                    .content
                    .alternatives
                    .iter()
                    .map(|a| a.text.as_str())
                    .collect();
                let indices = resolve_single_match(id, field, &items, opts)?;
                let text = entry.spec.content.alternatives[indices].text.clone();
                entry.spec.content.alternatives[indices].status = new_status;
                text
            }
            _ => anyhow::bail!("Unknown field for tick: {field}"),
        };

        write_adr(&entry.path, &entry.spec, op)?;
        if !op.is_preview() {
            ui::ticked(&ticked_text, new_status.as_ref());
        }
    } else {
        anyhow::bail!("Tick only works for work items and ADRs: {id}");
    }

    Ok(vec![])
}

/// Resolve match result to a single index (tick doesn't allow multiple)
fn resolve_single_match(
    id: &str,
    field: &str,
    items: &[&str],
    opts: &MatchOptions,
) -> anyhow::Result<usize> {
    if items.is_empty() {
        anyhow::bail!("Field {}.{} is empty", id, field);
    }

    match find_matches(items, opts)? {
        MatchResult::None => {
            let pattern = opts.pattern.unwrap_or("<index>");
            anyhow::bail!("No items match '{}' in {}.{}", pattern, id, field);
        }
        MatchResult::Single(idx) => Ok(idx),
        MatchResult::Multiple(indices) => {
            let pattern = opts.pattern.unwrap_or("");
            let mut msg = format!(
                "{} items match '{}' in {}.{}:\n",
                indices.len(),
                pattern,
                id,
                field
            );
            for &i in &indices {
                msg.push_str(&format!("  [{}] {}\n", i, items[i]));
            }
            msg.push_str("\nUse more specific pattern or --at <index> to select one");
            anyhow::bail!("{}", msg);
        }
    }
}
