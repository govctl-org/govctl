//! Render command implementation.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::load_rfcs;
use crate::parse::{load_adrs, load_work_items};
use crate::render::{write_adr_md, write_rfc, write_work_item_md};
use crate::ui;
use std::path::Path;

mod changelog;
mod show;
pub use changelog::render_changelog;
pub use show::{show_adr, show_clause, show_rfc, show_work};

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
