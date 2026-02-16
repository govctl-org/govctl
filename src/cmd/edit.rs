//! Edit command implementation - modify artifacts.
//!
//! Implements [[ADR-0007]] ergonomic array field matching for remove and tick commands.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::{find_clause_json, find_rfc_json};
use crate::model::{AdrEntry, ClauseSpec, RfcSpec, WorkItemEntry};
use crate::parse::{load_adrs, load_work_items, write_adr, write_work_item};
use crate::ui;
use crate::write::{
    WriteOp, delete_file, read_clause, read_rfc, update_clause_field, update_rfc_field,
    write_clause, write_rfc,
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

/// Normalize field name aliases to canonical form.
///
/// Supports short aliases for commonly used fields:
/// - `ac` → `acceptance_criteria`
/// - `alt` → `alternatives`
/// - `desc` → `description`
fn normalize_field(field: &str) -> &str {
    match field {
        "ac" => "acceptance_criteria",
        "alt" => "alternatives",
        "desc" => "description",
        _ => field,
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
            return Err(Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!(
                    "Index {} out of range (array has {} items)",
                    idx,
                    items.len()
                ),
                "array",
            )
            .into());
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
        (None, false) => Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Provide a value or use --stdin",
            "input",
        )
        .into()),
        (Some(_), true) => Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot use both value and --stdin",
            "input",
        )
        .into()),
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
        (None, None, false) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0801MissingRequiredArg,
                "Provide --text, --text-file, or --stdin",
                "input",
            )
            .into());
        }
        _ => unreachable!("clap arg group ensures mutual exclusivity"),
    };

    clause.text = new_text;
    write_clause(&clause_path, &clause, op, None)?;

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
    let field = normalize_field(field);
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
            write_clause(&path, &data, op, None)?;
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
            write_rfc(&path, &data, op, None)?;
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
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0803UnknownField,
                        format!("Unknown ADR field: {field}"),
                        id,
                    )
                    .into());
                }
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
                "notes" | "content.notes" => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0804FieldNotEditable,
                        "Use 'add' to append notes and 'remove' to delete them",
                        id,
                    )
                    .into());
                }
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0803UnknownField,
                        format!("Unknown work item field: {field}"),
                        id,
                    )
                    .into());
                }
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
    let field = field.map(normalize_field);
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
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown clause field: {f}"),
                            id,
                        )
                        .into());
                    }
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
                    "refs" => rfc.refs.join(", "),
                    "supersedes" => rfc.supersedes.unwrap_or_default(),
                    "created" => rfc.created,
                    "updated" => rfc.updated.unwrap_or_default(),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown RFC field: {f}"),
                            id,
                        )
                        .into());
                    }
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
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown ADR field: {f}"),
                            id,
                        )
                        .into());
                    }
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
                    "notes" => entry.spec.content.notes.join("\n"),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown work item field: {f}"),
                            id,
                        )
                        .into());
                    }
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
    category_override: Option<crate::model::ChangelogCategory>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let field = normalize_field(field);
    let value = resolve_value(value, stdin)?;
    let value = value.as_str();

    match ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))? {
        ArtifactType::Rfc => {
            let LoadedRfc { path, mut data } = load_rfc(config, id)?;

            match field {
                "owners" => push_unique(&mut data.owners, value),
                "refs" => push_unique(&mut data.refs, value),
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0810CannotAddToField,
                        format!("Cannot add to field: {field} (not an array or unsupported)"),
                        id,
                    )
                    .into());
                }
            }

            write_rfc(&path, &data, op, None)?;
        }
        ArtifactType::Clause => {
            let LoadedClause { path, mut data } = load_clause(config, id)?;

            match field {
                "anchors" => push_unique(&mut data.anchors, value),
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0810CannotAddToField,
                        format!("Cannot add to field: {field} (not an array or unsupported)"),
                        id,
                    )
                    .into());
                }
            }

            write_clause(&path, &data, op, None)?;
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
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0810CannotAddToField,
                        format!("Cannot add to field: {field} (not an array or unsupported)"),
                        id,
                    )
                    .into());
                }
            }

            write_adr(&entry.path, &entry.spec, op)?;
        }
        ArtifactType::WorkItem => {
            let mut entry = load_work_item(config, id)?;

            match field {
                "refs" => push_unique(&mut entry.spec.govctl.refs, value),
                "acceptance_criteria" => {
                    use crate::model::ChecklistItem;
                    use crate::write::parse_changelog_change;
                    // Parse prefix per ADR-0012/ADR-0013
                    let parsed = parse_changelog_change(value)?;

                    // Determine final category: explicit override > parsed prefix > error
                    let final_category = if let Some(cat) = category_override {
                        cat
                    } else if parsed.explicit {
                        parsed.category
                    } else {
                        // No explicit category specified - require prefix or --category
                        return Err(Diagnostic::new(
                            DiagnosticCode::E0408WorkCriteriaMissingCategory,
                            format!(
                                "Acceptance criteria requires category. Use prefix (e.g., 'fix: {}') or --category",
                                parsed.message
                            ),
                            id,
                        )
                        .into());
                    };

                    if !entry
                        .spec
                        .content
                        .acceptance_criteria
                        .iter()
                        .any(|c| c.text == parsed.message)
                    {
                        entry
                            .spec
                            .content
                            .acceptance_criteria
                            .push(ChecklistItem::with_category(
                                &parsed.message,
                                final_category,
                            ));
                    }
                }
                "notes" => {
                    if !entry.spec.content.notes.contains(&value.to_string()) {
                        entry.spec.content.notes.push(value.to_string());
                    }
                }
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0810CannotAddToField,
                        format!("Cannot add to field: {field} (not an array or unsupported)"),
                        id,
                    )
                    .into());
                }
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
    let field = normalize_field(field);
    match ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))? {
        ArtifactType::Rfc => {
            let LoadedRfc { path, mut data } = load_rfc(config, id)?;

            let removed = match field {
                "owners" => remove_matching_strings(&mut data.owners, id, field, opts)?,
                "refs" => remove_matching_strings(&mut data.refs, id, field, opts)?,
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0811CannotRemoveFromField,
                        format!("Cannot remove from field: {field}"),
                        id,
                    )
                    .into());
                }
            };

            write_rfc(&path, &data, op, None)?;
            notify_removed(id, field, &removed, op);
        }
        ArtifactType::Clause => {
            let LoadedClause { path, mut data } = load_clause(config, id)?;

            let removed = match field {
                "anchors" => remove_matching_strings(&mut data.anchors, id, field, opts)?,
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0811CannotRemoveFromField,
                        format!("Cannot remove from field: {field}"),
                        id,
                    )
                    .into());
                }
            };

            write_clause(&path, &data, op, None)?;
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
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0811CannotRemoveFromField,
                        format!("Cannot remove from field: {field}"),
                        id,
                    )
                    .into());
                }
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
                "notes" => remove_matching_strings(&mut entry.spec.content.notes, id, field, opts)?,
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0811CannotRemoveFromField,
                        format!("Cannot remove from field: {field}"),
                        id,
                    )
                    .into());
                }
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
        return Err(Diagnostic::new(
            DiagnosticCode::E0812FieldEmpty,
            format!("Field {}.{} is empty", id, field),
            id,
        )
        .into());
    }

    match find_matches(items, opts)? {
        MatchResult::None => {
            let pattern = opts.pattern.unwrap_or("<index>");
            Err(Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!("No items match '{}' in {}.{}", pattern, id, field),
                id,
            )
            .into())
        }
        MatchResult::Single(idx) => Ok(vec![idx]),
        MatchResult::Multiple(indices) => {
            if opts.all {
                Ok(indices)
            } else {
                let pattern = opts.pattern.unwrap_or("");
                Err(Diagnostic::new(
                    DiagnosticCode::E0807AmbiguousMatch,
                    format_multiple_match_error(id, field, pattern, items, &indices),
                    id,
                )
                .into())
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

    let field = normalize_field(field);
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
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0803UnknownField,
                        format!("Unknown field for tick: {field}"),
                        id,
                    )
                    .into());
                }
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
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0803UnknownField,
                        format!("Unknown field for tick: {field}"),
                        id,
                    )
                    .into());
                }
            };

            write_adr(&entry.path, &entry.spec, op)?;
            (ticked_text, status_str)
        }
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0813SupersedeNotSupported,
                format!("Tick only works for work items and ADRs: {id}"),
                id,
            )
            .into());
        }
    };

    if !op.is_preview() {
        ui::ticked(&ticked_text, &status_str);
    }

    Ok(vec![])
}

/// Delete a clause from a draft RFC.
///
/// Safety: Only clauses in draft RFCs can be deleted per [[RFC-0000:C-STATUS-LIFECYCLE]].
/// Atomically removes the clause JSON file and updates the parent RFC's clauses array.
pub fn delete_clause(
    config: &Config,
    clause_id: &str,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    use crate::model::RfcStatus;
    

    // Parse clause_id (RFC-0001:C-NAME)
    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            "Invalid clause ID format. Expected RFC-NNNN:C-NAME",
            clause_id,
        )
        .into());
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];

    // Load RFC and check status
    let rfc_loaded = load_rfc(config, rfc_id)?;

    // Safety check: Only draft RFCs allow clause deletion per [[RFC-0000:C-STATUS-LIFECYCLE]]
    if rfc_loaded.data.status != RfcStatus::Draft {
        return Err(Diagnostic::new(
            DiagnosticCode::E0110RfcInvalidId,
            format!(
                "Cannot delete clause: {} is {}. Only draft RFCs allow clause deletion.",
                rfc_id,
                rfc_loaded.data.status.as_ref()
            ),
            clause_id,
        )
        .into());
    }

    // Verify clause exists
    let clause_path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{}.json", clause_name));

    if !clause_path.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Clause not found: {}", clause_id),
            clause_id,
        )
        .into());
    }

    // Confirmation prompt (unless force or dry-run)
    if !force && !op.is_preview() {
        use std::io::{self, Write};
        print!("Delete clause {} from {}? [y/N] ", clause_name, rfc_id);
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            ui::info("Deletion cancelled");
            return Ok(vec![]);
        }
    }

    // Update RFC to remove clause from sections
    let mut rfc = rfc_loaded.data.clone();
    let clause_rel_path = format!("clauses/{}.json", clause_name);

    let mut removed = false;
    for section in &mut rfc.sections {
        if let Some(pos) = section.clauses.iter().position(|c| c == &clause_rel_path) {
            section.clauses.remove(pos);
            removed = true;
            break;
        }
    }

    if !removed {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!(
                "Clause {} not found in any section of {}",
                clause_name, rfc_id
            ),
            clause_id,
        )
        .into());
    }

    // Write updated RFC
    write_rfc(&rfc_loaded.path, &rfc, op, Some(&config.display_path(&rfc_loaded.path)))?;

    // Delete clause file
    delete_file(&clause_path, op, Some(&config.display_path(&clause_path)))?;

    if !op.is_preview() {
        ui::success(format!("Deleted clause {}", clause_id));
    }

    Ok(vec![])
}

/// Delete a queued work item.
///
/// Safety: Only work items in queue status can be deleted.
/// Work items that have been activated should use 'cancelled' status instead.
pub fn delete_work_item(
    config: &Config,
    id: &str,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    use crate::load::load_project_with_warnings;
    use crate::model::WorkItemStatus;

    // Load work item
    let entry = load_work_item(config, id)?;
    let wi = &entry.spec;

    // Safety check: Only queue-status work items can be deleted
    if wi.govctl.status != WorkItemStatus::Queue {
        return Err(Diagnostic::new(
            DiagnosticCode::E0402WorkNotFound,
            format!(
                "Cannot delete work item: {} is {}. Only queued work items can be deleted. Use 'mv {} cancelled' for active items.",
                wi.govctl.id,
                wi.govctl.status.as_ref(),
                wi.govctl.id
            ),
            id,
        )
        .into());
    }

    // Check for references in other artifacts
    let load_result = match load_project_with_warnings(config) {
        Ok(result) => result,
        Err(_) => {
            // If project fails to load, proceed anyway (deletion might help fix it)
            return proceed_with_deletion(config, &entry.path, &wi.govctl.id, force, op);
        }
    };

    let index = &load_result.index;
    let mut referenced_by = Vec::new();

    // Check RFCs
    for rfc in &index.rfcs {
        if rfc.rfc.refs.contains(&wi.govctl.id) {
            referenced_by.push(rfc.rfc.rfc_id.clone());
        }
    }

    // Check ADRs
    for adr in &index.adrs {
        if adr.spec.govctl.refs.contains(&wi.govctl.id) {
            referenced_by.push(adr.spec.govctl.id.clone());
        }
    }

    // Check other work items
    for other_wi in &index.work_items {
        if other_wi.spec.govctl.id != wi.govctl.id
            && other_wi.spec.govctl.refs.contains(&wi.govctl.id)
        {
            referenced_by.push(other_wi.spec.govctl.id.clone());
        }
    }

    if !referenced_by.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0404WorkRefNotFound,
            format!(
                "Cannot delete work item: {} is referenced by: {}. Remove references first.",
                wi.govctl.id,
                referenced_by.join(", ")
            ),
            id,
        )
        .into());
    }

    proceed_with_deletion(config, &entry.path, &wi.govctl.id, force, op)
}

/// Helper function to proceed with deletion after checks pass
fn proceed_with_deletion(
    config: &Config,
    path: &std::path::Path,
    id: &str,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Confirmation prompt (unless force or dry-run)
    if !force && !op.is_preview() {
        use std::io::{self, Write};
        print!("Delete work item {}? [y/N] ", id);
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            ui::info("Deletion cancelled");
            return Ok(vec![]);
        }
    }

    // Delete file
    delete_file(path, op, Some(&config.display_path(path)))?;

    if !op.is_preview() {
        ui::success(format!("Deleted work item {}", id));
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
        Err(Diagnostic::new(DiagnosticCode::E0807AmbiguousMatch, msg, id).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ArtifactType::from_id Tests
    // =========================================================================

    #[test]
    fn test_artifact_type_clause() {
        assert_eq!(
            ArtifactType::from_id("RFC-0001:C-NAME"),
            Some(ArtifactType::Clause)
        );
        assert_eq!(
            ArtifactType::from_id("RFC-0000:C-SUMMARY"),
            Some(ArtifactType::Clause)
        );
    }

    #[test]
    fn test_artifact_type_rfc() {
        assert_eq!(ArtifactType::from_id("RFC-0001"), Some(ArtifactType::Rfc));
        assert_eq!(ArtifactType::from_id("RFC-9999"), Some(ArtifactType::Rfc));
    }

    #[test]
    fn test_artifact_type_adr() {
        assert_eq!(ArtifactType::from_id("ADR-0001"), Some(ArtifactType::Adr));
        assert_eq!(ArtifactType::from_id("ADR-0007"), Some(ArtifactType::Adr));
    }

    #[test]
    fn test_artifact_type_work_item_by_prefix() {
        assert_eq!(
            ArtifactType::from_id("WI-2026-01-17-001"),
            Some(ArtifactType::WorkItem)
        );
    }

    #[test]
    fn test_artifact_type_work_item_by_hyphen() {
        // Any ID with hyphen that doesn't match RFC/ADR/Clause is WorkItem
        assert_eq!(
            ArtifactType::from_id("2026-01-17-add-tests"),
            Some(ArtifactType::WorkItem)
        );
    }

    #[test]
    fn test_artifact_type_unknown() {
        assert_eq!(ArtifactType::from_id("UNKNOWN"), None);
        assert_eq!(ArtifactType::from_id("foo"), None);
    }

    // =========================================================================
    // find_matches Tests - Substring (Default)
    // =========================================================================

    #[test]
    fn test_find_matches_substring_single() {
        let items = vec!["apple", "banana", "cherry"];
        let opts = MatchOptions {
            pattern: Some("nan"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_substring_case_insensitive() {
        let items = vec!["Apple", "BANANA", "Cherry"];
        let opts = MatchOptions {
            pattern: Some("banana"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_substring_multiple() {
        let items = vec!["test-one", "test-two", "other"];
        let opts = MatchOptions {
            pattern: Some("test"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Multiple(indices) => assert_eq!(indices, vec![0, 1]),
            _ => panic!("Expected multiple matches"),
        }
    }

    #[test]
    fn test_find_matches_substring_none() {
        let items = vec!["apple", "banana", "cherry"];
        let opts = MatchOptions {
            pattern: Some("xyz"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::None => {}
            _ => panic!("Expected no match"),
        }
    }

    // =========================================================================
    // find_matches Tests - Exact Match
    // =========================================================================

    #[test]
    fn test_find_matches_exact_match() {
        let items = vec!["test", "testing", "test"];
        let opts = MatchOptions {
            pattern: Some("test"),
            exact: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Multiple(indices) => assert_eq!(indices, vec![0, 2]),
            _ => panic!("Expected multiple matches"),
        }
    }

    #[test]
    fn test_find_matches_exact_case_sensitive() {
        let items = vec!["Test", "test", "TEST"];
        let opts = MatchOptions {
            pattern: Some("test"),
            exact: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_exact_no_match() {
        let items = vec!["testing", "tested"];
        let opts = MatchOptions {
            pattern: Some("test"),
            exact: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::None => {}
            _ => panic!("Expected no match"),
        }
    }

    // =========================================================================
    // find_matches Tests - Index-based
    // =========================================================================

    #[test]
    fn test_find_matches_at_positive() {
        let items = vec!["a", "b", "c"];
        let opts = MatchOptions {
            at: Some(1),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_zero() {
        let items = vec!["first", "second"];
        let opts = MatchOptions {
            at: Some(0),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 0),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_negative() {
        let items = vec!["a", "b", "c"];
        let opts = MatchOptions {
            at: Some(-1),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 2), // last item
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_negative_two() {
        let items = vec!["a", "b", "c", "d"];
        let opts = MatchOptions {
            at: Some(-2),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 2), // second to last
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_out_of_range() {
        let items = vec!["a", "b"];
        let opts = MatchOptions {
            at: Some(5),
            ..Default::default()
        };
        assert!(find_matches(&items, &opts).is_err());
    }

    #[test]
    fn test_find_matches_at_negative_out_of_range() {
        let items = vec!["a", "b"];
        let opts = MatchOptions {
            at: Some(-5),
            ..Default::default()
        };
        assert!(find_matches(&items, &opts).is_err());
    }

    // =========================================================================
    // find_matches Tests - Regex
    // =========================================================================

    #[test]
    fn test_find_matches_regex_single() {
        let items = vec!["RFC-0001", "ADR-0001", "WI-001"];
        let opts = MatchOptions {
            pattern: Some("RFC-.*"),
            regex: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 0),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_regex_multiple() {
        let items = vec!["test-1", "test-2", "other"];
        let opts = MatchOptions {
            pattern: Some("test-\\d+"),
            regex: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Multiple(indices) => assert_eq!(indices, vec![0, 1]),
            _ => panic!("Expected multiple matches"),
        }
    }

    #[test]
    fn test_find_matches_regex_invalid() {
        let items = vec!["a", "b"];
        let opts = MatchOptions {
            pattern: Some("[invalid"),
            regex: true,
            ..Default::default()
        };
        assert!(find_matches(&items, &opts).is_err());
    }

    // =========================================================================
    // remove_indices Tests
    // =========================================================================

    #[test]
    fn test_remove_indices_single() {
        let mut items = vec!["a", "b", "c"];
        remove_indices(&mut items, &[1]);
        assert_eq!(items, vec!["a", "c"]);
    }

    #[test]
    fn test_remove_indices_multiple() {
        let mut items = vec!["a", "b", "c", "d"];
        remove_indices(&mut items, &[1, 3]);
        assert_eq!(items, vec!["a", "c"]);
    }

    #[test]
    fn test_remove_indices_all() {
        let mut items = vec!["a", "b", "c"];
        remove_indices(&mut items, &[0, 1, 2]);
        assert!(items.is_empty());
    }

    #[test]
    fn test_remove_indices_preserves_order() {
        let mut items = vec!["1", "2", "3", "4", "5"];
        remove_indices(&mut items, &[0, 2, 4]);
        assert_eq!(items, vec!["2", "4"]);
    }

    #[test]
    fn test_remove_indices_empty() {
        let mut items = vec!["a", "b"];
        remove_indices(&mut items, &[]);
        assert_eq!(items, vec!["a", "b"]);
    }
}
