//! Edit command implementation - modify artifacts.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::{find_clause_json, find_rfc_json};
use crate::model::{AdrEntry, ClauseSpec, RfcSpec, WorkItemEntry};
use crate::parse::{load_adrs, load_work_items, write_adr, write_work_item};
use crate::ui;
use crate::write::{
    WriteOp, read_clause, read_rfc, update_clause_field, update_rfc_field, write_clause, write_rfc,
};
use anyhow::Context;
use regex::Regex;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Artifact type determined from ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType {
    Clause,
    Rfc,
    Adr,
    WorkItem,
}

impl ArtifactType {
    /// Parse artifact type from ID string
    pub fn from_id(id: &str) -> Option<Self> {
        if id.contains(':') {
            Some(Self::Clause)
        } else if id.starts_with("RFC-") {
            Some(Self::Rfc)
        } else if id.starts_with("ADR-") {
            Some(Self::Adr)
        } else if id.starts_with("WI-") || id.contains('-') {
            Some(Self::WorkItem)
        } else {
            None
        }
    }

    /// Error message for unknown artifact type
    pub fn unknown_error(id: &str) -> anyhow::Error {
        anyhow::anyhow!("Unknown artifact type: {id}")
    }
}

/// Loaded RFC with its path
pub struct LoadedRfc {
    pub path: PathBuf,
    pub data: RfcSpec,
}

/// Loaded Clause with its path
pub struct LoadedClause {
    pub path: PathBuf,
    pub data: ClauseSpec,
}

/// Load an RFC by ID
fn load_rfc(config: &Config, id: &str) -> anyhow::Result<LoadedRfc> {
    let path = find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;
    let data = read_rfc(&path)?;
    Ok(LoadedRfc { path, data })
}

/// Load a Clause by ID
fn load_clause(config: &Config, id: &str) -> anyhow::Result<LoadedClause> {
    let path =
        find_clause_json(config, id).ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;
    let data = read_clause(&path)?;
    Ok(LoadedClause { path, data })
}

/// Load an ADR by ID
fn load_adr(config: &Config, id: &str) -> anyhow::Result<AdrEntry> {
    load_adrs(config)?
        .into_iter()
        .find(|a| a.spec.govctl.id == id)
        .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))
}

/// Load a Work Item by ID
fn load_work_item(config: &Config, id: &str) -> anyhow::Result<WorkItemEntry> {
    load_work_items(config)?
        .into_iter()
        .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
        .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))
}

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

    match ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))? {
        ArtifactType::Clause => {
            crate::validate::validate_field(
                config,
                id,
                crate::validate::ArtifactKind::Clause,
                field,
                value,
            )?;

            let LoadedClause { path, mut data } = load_clause(config, id)?;
            update_clause_field(&mut data, field, value)?;
            write_clause(&path, &data, op)?;
        }
        ArtifactType::Rfc => {
            crate::validate::validate_field(
                config,
                id,
                crate::validate::ArtifactKind::Rfc,
                field,
                value,
            )?;

            let LoadedRfc { path, mut data } = load_rfc(config, id)?;
            update_rfc_field(&mut data, field, value)?;
            write_rfc(&path, &data, op)?;
        }
        ArtifactType::Adr => {
            let mut entry = load_adr(config, id)?;

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
        }
        ArtifactType::WorkItem => {
            let mut entry = load_work_item(config, id)?;

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
        }
    }

    if !op.is_preview() {
        ui::field_set(id, field, value);
    }

    Ok(vec![])
}

/// Get a field value
pub fn get_field(
    config: &Config,
    id: &str,
    field: Option<&str>,
) -> anyhow::Result<Vec<Diagnostic>> {
    match ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))? {
        ArtifactType::Clause => {
            let LoadedClause { data: clause, .. } = load_clause(config, id)?;

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
        }
        ArtifactType::Rfc => {
            let LoadedRfc { data: rfc, .. } = load_rfc(config, id)?;

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
        }
        ArtifactType::Adr => {
            let entry = load_adr(config, id)?;

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
                    "alternatives" => format_status_items(
                        &entry.spec.content.alternatives,
                        |a| a.status.as_ref(),
                        |a| &a.text,
                    ),
                    _ => anyhow::bail!("Unknown field: {f}"),
                };
                println!("{value}");
            } else {
                println!("{}", toml::to_string_pretty(&entry.spec)?);
            }
        }
        ArtifactType::WorkItem => {
            let entry = load_work_item(config, id)?;

            if let Some(f) = field {
                let value = match f {
                    "status" => entry.spec.govctl.status.as_ref().to_string(),
                    "title" => entry.spec.govctl.title,
                    "started" => entry.spec.govctl.started.unwrap_or_default(),
                    "completed" => entry.spec.govctl.completed.unwrap_or_default(),
                    "refs" => entry.spec.govctl.refs.join(", "),
                    "description" => entry.spec.content.description,
                    "acceptance_criteria" => format_status_items(
                        &entry.spec.content.acceptance_criteria,
                        |c| c.status.as_ref(),
                        |c| &c.text,
                    ),
                    "decisions" => format_status_items(
                        &entry.spec.content.decisions,
                        |d| d.status.as_ref(),
                        |d| &d.text,
                    ),
                    "notes" => entry.spec.content.notes,
                    _ => anyhow::bail!("Unknown field: {f}"),
                };
                println!("{value}");
            } else {
                println!("{}", toml::to_string_pretty(&entry.spec)?);
            }
        }
    }

    Ok(vec![])
}

/// Format items with status as "[status] text" lines
fn format_status_items<T, S, X>(items: &[T], get_status: S, get_text: X) -> String
where
    S: Fn(&T) -> &str,
    X: Fn(&T) -> &str,
{
    items
        .iter()
        .map(|item| format!("[{}] {}", get_status(item), get_text(item)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Add unique value to a string array (no-op if already present)
fn push_unique(vec: &mut Vec<String>, value: &str) {
    if !vec.contains(&value.to_string()) {
        vec.push(value.to_string());
    }
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

    match ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))? {
        ArtifactType::Rfc => {
            let LoadedRfc { path, mut data } = load_rfc(config, id)?;

            match field {
                "owners" => push_unique(&mut data.owners, value),
                _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
            }

            write_rfc(&path, &data, op)?;
        }
        ArtifactType::Clause => {
            let LoadedClause { path, mut data } = load_clause(config, id)?;

            match field {
                "anchors" => push_unique(&mut data.anchors, value),
                _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
            }

            write_clause(&path, &data, op)?;
        }
        ArtifactType::Adr => {
            let mut entry = load_adr(config, id)?;

            match field {
                "refs" => push_unique(&mut entry.spec.govctl.refs, value),
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
        }
        ArtifactType::WorkItem => {
            let mut entry = load_work_item(config, id)?;

            match field {
                "refs" => push_unique(&mut entry.spec.govctl.refs, value),
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
        }
    }

    if !op.is_preview() {
        ui::field_added(id, field, value);
    }

    Ok(vec![])
}

/// Remove matching items from a Vec<String> and return the removed items
fn remove_matching_strings(
    vec: &mut Vec<String>,
    id: &str,
    field: &str,
    opts: &MatchOptions,
) -> anyhow::Result<Vec<String>> {
    let items: Vec<&str> = vec.iter().map(|s| s.as_str()).collect();
    let to_remove = resolve_matches(id, field, &items, opts)?;
    let removed: Vec<String> = to_remove.iter().map(|&i| vec[i].clone()).collect();
    remove_indices(vec, &to_remove);
    Ok(removed)
}

/// Remove matching items from a Vec<T> where T has a text field, return removed texts
fn remove_matching_items<T, F>(
    vec: &mut Vec<T>,
    get_text: F,
    id: &str,
    field: &str,
    opts: &MatchOptions,
) -> anyhow::Result<Vec<String>>
where
    F: Fn(&T) -> &str,
{
    let items: Vec<&str> = vec.iter().map(&get_text).collect();
    let to_remove = resolve_matches(id, field, &items, opts)?;
    let removed: Vec<String> = to_remove
        .iter()
        .map(|&i| get_text(&vec[i]).to_string())
        .collect();
    remove_indices(vec, &to_remove);
    Ok(removed)
}

/// Notify UI about removed items
fn notify_removed(id: &str, field: &str, removed: &[String], op: WriteOp) {
    if !op.is_preview() {
        for item in removed {
            ui::field_removed(id, field, item);
        }
    }
}

/// Remove matching items from an array field (per ADR-0007)
pub fn remove_from_field(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    match ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))? {
        ArtifactType::Rfc => {
            let LoadedRfc { path, mut data } = load_rfc(config, id)?;

            let removed = match field {
                "owners" => remove_matching_strings(&mut data.owners, id, field, opts)?,
                _ => anyhow::bail!("Cannot remove from field: {field}"),
            };

            write_rfc(&path, &data, op)?;
            notify_removed(id, field, &removed, op);
        }
        ArtifactType::Clause => {
            let LoadedClause { path, mut data } = load_clause(config, id)?;

            let removed = match field {
                "anchors" => remove_matching_strings(&mut data.anchors, id, field, opts)?,
                _ => anyhow::bail!("Cannot remove from field: {field}"),
            };

            write_clause(&path, &data, op)?;
            notify_removed(id, field, &removed, op);
        }
        ArtifactType::Adr => {
            let mut entry = load_adr(config, id)?;

            let removed = match field {
                "refs" => remove_matching_strings(&mut entry.spec.govctl.refs, id, field, opts)?,
                "alternatives" => remove_matching_items(
                    &mut entry.spec.content.alternatives,
                    |a| &a.text,
                    id,
                    field,
                    opts,
                )?,
                _ => anyhow::bail!("Cannot remove from field: {field}"),
            };

            write_adr(&entry.path, &entry.spec, op)?;
            notify_removed(id, field, &removed, op);
        }
        ArtifactType::WorkItem => {
            let mut entry = load_work_item(config, id)?;

            let removed = match field {
                "refs" => remove_matching_strings(&mut entry.spec.govctl.refs, id, field, opts)?,
                "acceptance_criteria" => remove_matching_items(
                    &mut entry.spec.content.acceptance_criteria,
                    |c| &c.text,
                    id,
                    field,
                    opts,
                )?,
                "decisions" => remove_matching_items(
                    &mut entry.spec.content.decisions,
                    |d| &d.text,
                    id,
                    field,
                    opts,
                )?,
                _ => anyhow::bail!("Cannot remove from field: {field}"),
            };

            write_work_item(&entry.path, &entry.spec, op)?;
            notify_removed(id, field, &removed, op);
        }
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

/// Find single match and update status, returning the matched text
fn tick_checklist<T, S, F>(
    items: &mut [T],
    get_text: F,
    set_status: S,
    id: &str,
    field: &str,
    opts: &MatchOptions,
) -> anyhow::Result<String>
where
    F: Fn(&T) -> &str,
    S: FnOnce(&mut T),
{
    let texts: Vec<&str> = items.iter().map(&get_text).collect();
    let idx = resolve_single_match(id, field, &texts, opts)?;
    let text = get_text(&items[idx]).to_string();
    set_status(&mut items[idx]);
    Ok(text)
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

    let artifact_type = ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))?;

    let (ticked_text, status_str): (String, String) = match artifact_type {
        ArtifactType::WorkItem => {
            let mut entry = load_work_item(config, id)?;

            let new_status = match status {
                crate::TickStatus::Done => ChecklistStatus::Done,
                crate::TickStatus::Pending => ChecklistStatus::Pending,
                crate::TickStatus::Cancelled => ChecklistStatus::Cancelled,
            };
            let status_str = new_status.as_ref().to_string();

            let ticked_text = match field {
                "acceptance_criteria" => tick_checklist(
                    &mut entry.spec.content.acceptance_criteria,
                    |c| &c.text,
                    |c| c.status = new_status,
                    id,
                    field,
                    opts,
                )?,
                "decisions" => tick_checklist(
                    &mut entry.spec.content.decisions,
                    |d| &d.text,
                    |d| d.status = new_status,
                    id,
                    field,
                    opts,
                )?,
                _ => anyhow::bail!("Unknown field for tick: {field}"),
            };

            write_work_item(&entry.path, &entry.spec, op)?;
            (ticked_text, status_str)
        }
        ArtifactType::Adr => {
            let mut entry = load_adr(config, id)?;

            let new_status = match status {
                crate::TickStatus::Done => AlternativeStatus::Accepted,
                crate::TickStatus::Pending => AlternativeStatus::Considered,
                crate::TickStatus::Cancelled => AlternativeStatus::Rejected,
            };
            let status_str = new_status.as_ref().to_string();

            let ticked_text = match field {
                "alternatives" => tick_checklist(
                    &mut entry.spec.content.alternatives,
                    |a| &a.text,
                    |a| a.status = new_status,
                    id,
                    field,
                    opts,
                )?,
                _ => anyhow::bail!("Unknown field for tick: {field}"),
            };

            write_adr(&entry.path, &entry.spec, op)?;
            (ticked_text, status_str)
        }
        _ => anyhow::bail!("Tick only works for work items and ADRs: {id}"),
    };

    if !op.is_preview() {
        ui::ticked(&ticked_text, &status_str);
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
    let indices = resolve_matches(id, field, items, opts)?;
    if indices.len() == 1 {
        Ok(indices[0])
    } else {
        // Multiple matches not allowed for tick operations
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
