//! Edit command implementation - modify artifacts.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::{find_clause_json, find_rfc_json};
use crate::write::{read_clause, read_rfc, update_clause_field, update_rfc_field, write_clause, write_rfc};
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
    let clause_path = find_clause_json(config, clause_id).ok_or_else(|| {
        anyhow::anyhow!("Clause not found: {clause_id}")
    })?;

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
    // Determine if it's an RFC, clause, or ADR
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
        let rfc_path = find_rfc_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

        let mut rfc = read_rfc(&rfc_path)?;
        update_rfc_field(&mut rfc, field, value)?;
        write_rfc(&rfc_path, &rfc)?;

        eprintln!("Set {id}.{field} = {value}");
    } else if id.starts_with("ADR-") {
        // It's an ADR - need to update frontmatter
        let adr = crate::parse::load_adrs(config)?
            .into_iter()
            .find(|a| a.meta.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        let mut meta = adr.meta;

        match field {
            "status" => {
                meta.status = serde_json::from_str(&format!("\"{value}\""))
                    .map_err(|_| anyhow::anyhow!("Invalid status: {value}"))?;
            }
            "title" => meta.title = value.to_string(),
            _ => anyhow::bail!("Unknown ADR field: {field}"),
        }

        let wrapper = crate::model::PhaseOsWrapper {
            phaseos: meta,
            ext: None,
        };
        crate::parse::update_frontmatter(&adr.path, &wrapper)?;

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
        let rfc_path = find_rfc_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;

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
        let adr = crate::parse::load_adrs(config)?
            .into_iter()
            .find(|a| a.meta.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))?;

        if let Some(f) = field {
            let value = match f {
                "status" => adr.meta.status.as_ref().to_string(),
                "title" => adr.meta.title,
                "date" => adr.meta.date,
                "superseded_by" => adr.meta.superseded_by.unwrap_or_default(),
                _ => anyhow::bail!("Unknown field: {f}"),
            };
            println!("{value}");
        } else {
            println!("{}", serde_yaml::to_string(&adr.meta)?);
        }
    } else {
        anyhow::bail!("Unknown artifact type: {id}");
    }

    Ok(vec![])
}
