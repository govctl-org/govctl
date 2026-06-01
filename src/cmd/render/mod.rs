//! Render command implementation.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
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

struct RenderSelection<'a> {
    id: Option<&'a str>,
    dry_run: bool,
    summary_label: &'a str,
}

fn render_selected<T, Empty, NotFound, Id, Write>(
    items: Vec<T>,
    selection: RenderSelection<'_>,
    empty: Empty,
    not_found: NotFound,
    item_id: Id,
    mut write: Write,
) -> DiagnosticResult<Diagnostics>
where
    Empty: FnOnce(),
    NotFound: FnOnce(&str) -> Diagnostic,
    Id: for<'a> Fn(&'a T) -> &'a str,
    Write: FnMut(&T) -> DiagnosticResult<()>,
{
    if items.is_empty() {
        empty();
        return Ok(vec![]);
    }

    let items_to_render: Vec<_> = if let Some(id) = selection.id {
        items
            .into_iter()
            .filter(|item| item_id(item) == id)
            .collect()
    } else {
        items
    };

    if items_to_render.is_empty()
        && let Some(id) = selection.id
    {
        return Err(not_found(id));
    }

    for item in &items_to_render {
        write(item)?;
    }

    if !selection.dry_run {
        ui::render_summary(items_to_render.len(), selection.summary_label);
    }

    Ok(vec![])
}

/// Render RFC markdown from JSON source
pub fn render(
    config: &Config,
    rfc_id: Option<&str>,
    dry_run: bool,
) -> DiagnosticResult<Diagnostics> {
    let rfcs = load_rfcs(config).map_err(Diagnostic::from)?;

    render_selected(
        rfcs,
        RenderSelection {
            id: rfc_id,
            dry_run,
            summary_label: "RFC",
        },
        || ui::not_found("RFC", &config.rfc_dir()),
        |id| {
            let scope = display_path_string(config, config.rfc_dir());
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {id}"),
                scope,
            )
        },
        |rfc| rfc.rfc.rfc_id.as_str(),
        |rfc| write_rfc(config, rfc, dry_run),
    )
}

/// Render ADRs to markdown
///
/// If `adr_id` is provided, renders only that ADR. Otherwise renders all.
pub fn render_adrs(
    config: &Config,
    adr_id: Option<&str>,
    dry_run: bool,
) -> DiagnosticResult<Diagnostics> {
    let adrs = load_adrs(config)?;

    render_selected(
        adrs,
        RenderSelection {
            id: adr_id,
            dry_run,
            summary_label: "ADR",
        },
        || ui::info("No ADRs found"),
        |id| {
            let scope = display_path_string(config, config.adr_dir());
            Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {id}"),
                scope,
            )
        },
        |adr| adr.spec.govctl.id.as_str(),
        |adr| write_adr_md(config, adr, dry_run),
    )
}

/// Render Work Items to markdown
///
/// If `work_id` is provided, renders only that work item. Otherwise renders all.
pub fn render_work_items(
    config: &Config,
    work_id: Option<&str>,
    dry_run: bool,
) -> DiagnosticResult<Diagnostics> {
    let items = load_work_items(config)?;

    render_selected(
        items,
        RenderSelection {
            id: work_id,
            dry_run,
            summary_label: "work item",
        },
        || ui::info("No work items found"),
        |id| {
            let scope = config
                .display_path(&config.work_dir())
                .display()
                .to_string();
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {id}"),
                scope,
            )
        },
        |item| item.spec.govctl.id.as_str(),
        |item| write_work_item_md(config, item, dry_run),
    )
}
