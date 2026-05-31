//! Render command implementation.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::load_rfcs;
use crate::parse::{load_adrs, load_work_items};
use crate::render::{write_adr_md, write_rfc, write_work_item_md};
use crate::ui;
use std::path::Path;

mod changelog;
pub use changelog::render_changelog;

fn display_path_string(config: &Config, path: impl AsRef<Path>) -> String {
    config.display_path(path.as_ref()).display().to_string()
}

/// Render RFC markdown from JSON source
pub fn render(
    config: &Config,
    rfc_id: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs = load_rfcs(config)
        .map_err(Diagnostic::from)
        .map_err(anyhow::Error::from)?;

    if rfcs.is_empty() {
        ui::not_found("RFC", &config.rfc_dir());
        return Ok(vec![]);
    }

    // Filter to specific RFC if provided
    let rfcs_to_render: Vec<_> = if let Some(id) = rfc_id {
        rfcs.into_iter().filter(|r| r.rfc.rfc_id == id).collect()
    } else {
        rfcs
    };

    if rfcs_to_render.is_empty()
        && let Some(id) = rfc_id
    {
        let scope = display_path_string(config, config.rfc_dir());
        return Err(Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {id}"),
            scope,
        )
        .into());
    }

    for rfc in &rfcs_to_render {
        write_rfc(config, rfc, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(rfcs_to_render.len(), "RFC");
    }

    Ok(vec![])
}

/// Render ADRs to markdown
///
/// If `adr_id` is provided, renders only that ADR. Otherwise renders all.
pub fn render_adrs(
    config: &Config,
    adr_id: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let adrs = load_adrs(config)?;

    if adrs.is_empty() {
        ui::info("No ADRs found");
        return Ok(vec![]);
    }

    // Filter to specific ADR if provided
    let adrs_to_render: Vec<_> = if let Some(id) = adr_id {
        adrs.into_iter()
            .filter(|a| a.spec.govctl.id == id)
            .collect()
    } else {
        adrs
    };

    if adrs_to_render.is_empty()
        && let Some(id) = adr_id
    {
        let scope = display_path_string(config, config.adr_dir());
        return Err(Diagnostic::new(
            DiagnosticCode::E0302AdrNotFound,
            format!("ADR not found: {id}"),
            scope,
        )
        .into());
    }

    for adr in &adrs_to_render {
        write_adr_md(config, adr, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(adrs_to_render.len(), "ADR");
    }

    Ok(vec![])
}

/// Render Work Items to markdown
///
/// If `work_id` is provided, renders only that work item. Otherwise renders all.
pub fn render_work_items(
    config: &Config,
    work_id: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let items = load_work_items(config)?;

    if items.is_empty() {
        ui::info("No work items found");
        return Ok(vec![]);
    }

    // Filter to specific work item if provided
    let items_to_render: Vec<_> = if let Some(id) = work_id {
        items
            .into_iter()
            .filter(|w| w.spec.govctl.id == id)
            .collect()
    } else {
        items
    };

    if items_to_render.is_empty()
        && let Some(id) = work_id
    {
        let scope = config
            .display_path(&config.work_dir())
            .display()
            .to_string();
        return Err(Diagnostic::new(
            DiagnosticCode::E0402WorkNotFound,
            format!("Work item not found: {id}"),
            scope,
        )
        .into());
    }

    for item in &items_to_render {
        write_work_item_md(config, item, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(items_to_render.len(), "work item");
    }

    Ok(vec![])
}

// =============================================================================
// Show Commands (stdout rendering per ADR-0022)
// =============================================================================

use crate::OutputFormat;
use crate::render::{expand_inline_refs, render_adr, render_clause, render_rfc, render_work_item};
use crate::terminal_md::render_terminal_md;

/// Show RFC content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_rfc(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs = load_rfcs(config)
        .map_err(Diagnostic::from)
        .map_err(anyhow::Error::from)?;

    let rfc = rfcs
        .into_iter()
        .find(|r| r.rfc.rfc_id == id)
        .ok_or_else(|| {
            let scope = display_path_string(config, config.rfc_dir());
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {id}"),
                scope,
            )
        })?;

    match output {
        OutputFormat::Json => {
            // Output the raw RFC data as JSON
            let json = serde_json::to_string_pretty(&rfc.rfc)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let raw = render_rfc(&rfc)?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}

/// Show ADR content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_adr(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let adrs = load_adrs(config)?;

    let adr = adrs
        .into_iter()
        .find(|a| a.spec.govctl.id == id)
        .ok_or_else(|| {
            let scope = display_path_string(config, config.adr_dir());
            Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {id}"),
                scope,
            )
        })?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&adr.spec)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let raw = render_adr(&adr)?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}

/// Show work item content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_work(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let items = load_work_items(config)?;

    let item = items
        .into_iter()
        .find(|w| w.spec.govctl.id == id)
        .ok_or_else(|| {
            let scope = config
                .display_path(&config.work_dir())
                .display()
                .to_string();
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {id}"),
                scope,
            )
        })?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&item.spec)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let raw = render_work_item(&item)?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}

/// Show clause content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_clause(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Parse clause ID: RFC-NNNN:C-NAME
    let parts: Vec<&str> = id.split(':').collect();
    if parts.len() != 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Invalid clause ID format: {id} (expected RFC-NNNN:C-NAME)"),
            id,
        )
        .into());
    }
    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfcs = load_rfcs(config)
        .map_err(Diagnostic::from)
        .map_err(anyhow::Error::from)?;

    let rfc = rfcs
        .into_iter()
        .find(|r| r.rfc.rfc_id == rfc_id)
        .ok_or_else(|| {
            let scope = display_path_string(config, config.rfc_dir());
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {rfc_id}"),
                scope,
            )
        })?;

    let clause = rfc
        .clauses
        .into_iter()
        .find(|c| c.spec.clause_id == clause_name)
        .ok_or_else(|| {
            let scope = config
                .display_path(&config.rfc_dir().join(rfc_id).join("clauses"))
                .display()
                .to_string();
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {id}"),
                scope,
            )
        })?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&clause.spec)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let mut raw = String::new();
            render_clause(&mut raw, rfc_id, &clause);
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}
