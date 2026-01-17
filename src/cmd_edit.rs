//! Edit command implementation - modify artifacts.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::{find_clause_json, find_rfc_json};
use crate::parse::{load_adrs, load_work_items, write_adr, write_work_item};
use crate::write::{
    read_clause, read_rfc, update_clause_field, update_rfc_field, write_clause, write_rfc,
};
use anyhow::Context;
use std::io::Read;
use std::path::Path;

/// Edit clause text
pub fn edit_clause(
    config: &Config,
    clause_id: &str,
    text: Option<&str>,
    text_file: Option<&Path>,
    text_stdin: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let clause_path = find_clause_json(config, clause_id)
        .ok_or_else(|| anyhow::anyhow!("Clause not found: {clause_id}"))?;

    let mut clause = read_clause(&clause_path)?;

    let new_text = match (text, text_file, text_stdin) {
        (Some(t), None, false) => t.to_string(),
        (None, Some(path), false) => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read text file: {}", path.display()))?,
        (None, None, true) => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read from stdin")?;
            // Trim trailing newline that HEREDOC adds
            buffer.trim_end_matches('\n').to_string()
        }
        (None, None, false) => anyhow::bail!("Provide --text, --text-file, or --text-stdin"),
        _ => unreachable!("clap arg group ensures mutual exclusivity"),
    };

    clause.text = new_text;
    write_clause(&clause_path, &clause)?;

    eprintln!("Updated clause: {clause_id}");
    Ok(vec![])
}

/// Set a field value
pub fn set_field(
    config: &Config,
    id: &str,
    field: &str,
    value: &str,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Determine if it's an RFC, clause, ADR, or Work Item
    if id.contains(':') {
        // It's a clause (RFC-0001:C-NAME)
        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        let mut clause = read_clause(&clause_path)?;
        update_clause_field(&mut clause, field, value)?;
        write_clause(&clause_path, &clause)?;

        eprintln!("Set {id}.{field} = {value}");
    } else if id.starts_with("RFC-") {
        // It's an RFC
        let rfc_path =
            find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

        let mut rfc = read_rfc(&rfc_path)?;
        update_rfc_field(&mut rfc, field, value)?;
        write_rfc(&rfc_path, &rfc)?;

        eprintln!("Set {id}.{field} = {value}");
    } else if id.starts_with("ADR-") {
        // It's an ADR - load, modify, write TOML
        let mut entry = load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        match field {
            "status" => {
                entry.spec.govctl.status = serde_json::from_str(&format!("\"{value}\""))
                    .map_err(|_| anyhow::anyhow!("Invalid status: {value}"))?;
            }
            "title" => entry.spec.govctl.title = value.to_string(),
            "date" => entry.spec.govctl.date = value.to_string(),
            "context" => entry.spec.content.context = value.to_string(),
            "decision" => entry.spec.content.decision = value.to_string(),
            "consequences" => entry.spec.content.consequences = value.to_string(),
            _ => anyhow::bail!("Unknown ADR field: {field}"),
        }

        write_adr(&entry.path, &entry.spec)?;
        eprintln!("Set {id}.{field} = {value}");
    } else if id.starts_with("WI-")
        || (id.contains("-") && !id.starts_with("RFC-") && !id.starts_with("ADR-"))
    {
        // It's a work item
        let mut entry = load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))?;

        match field {
            "status" => {
                entry.spec.govctl.status = serde_json::from_str(&format!("\"{value}\""))
                    .map_err(|_| anyhow::anyhow!("Invalid status: {value}"))?;
            }
            "title" => entry.spec.govctl.title = value.to_string(),
            "description" => entry.spec.content.description = value.to_string(),
            "notes" => entry.spec.content.notes = value.to_string(),
            _ => anyhow::bail!("Unknown work item field: {field}"),
        }

        write_work_item(&entry.path, &entry.spec)?;
        eprintln!("Set {id}.{field} = {value}");
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
                "start_date" => entry.spec.govctl.start_date.unwrap_or_default(),
                "done_date" => entry.spec.govctl.done_date.unwrap_or_default(),
                "refs" => entry.spec.govctl.refs.join(", "),
                "description" => entry.spec.content.description,
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
    value: &str,
) -> anyhow::Result<Vec<Diagnostic>> {
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

        write_rfc(&rfc_path, &rfc)?;
        eprintln!("Added '{value}' to {id}.{field}");
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

        write_clause(&clause_path, &clause)?;
        eprintln!("Added '{value}' to {id}.{field}");
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
            _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
        }

        write_adr(&entry.path, &entry.spec)?;
        eprintln!("Added '{value}' to {id}.{field}");
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
            _ => anyhow::bail!("Cannot add to field: {field} (not an array or unsupported)"),
        }

        write_work_item(&entry.path, &entry.spec)?;
        eprintln!("Added '{value}' to {id}.{field}");
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}

/// Remove a value from an array field
pub fn remove_from_field(
    config: &Config,
    id: &str,
    field: &str,
    value: &str,
) -> anyhow::Result<Vec<Diagnostic>> {
    if id.starts_with("RFC-") && !id.contains(':') {
        let rfc_path =
            find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

        let mut rfc = read_rfc(&rfc_path)?;

        match field {
            "owners" => {
                rfc.owners.retain(|o| o != value);
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }

        write_rfc(&rfc_path, &rfc)?;
        eprintln!("Removed '{value}' from {id}.{field}");
    } else if id.contains(':') {
        let clause_path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;

        let mut clause = read_clause(&clause_path)?;

        match field {
            "anchors" => {
                clause.anchors.retain(|a| a != value);
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }

        write_clause(&clause_path, &clause)?;
        eprintln!("Removed '{value}' from {id}.{field}");
    } else if id.starts_with("ADR-") {
        let mut entry = load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        match field {
            "refs" => {
                entry.spec.govctl.refs.retain(|r| r != value);
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }

        write_adr(&entry.path, &entry.spec)?;
        eprintln!("Removed '{value}' from {id}.{field}");
    } else if id.starts_with("WI-") || id.contains("-") {
        let mut entry = load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))?;

        match field {
            "refs" => {
                entry.spec.govctl.refs.retain(|r| r != value);
            }
            _ => anyhow::bail!("Cannot remove from field: {field}"),
        }

        write_work_item(&entry.path, &entry.spec)?;
        eprintln!("Removed '{value}' from {id}.{field}");
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}
