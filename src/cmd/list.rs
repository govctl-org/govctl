//! List command implementation.

use crate::ListTarget;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::model::WorkItemStatus;
use crate::ui::stdout_supports_color;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table, presets::UTF8_FULL};

/// Check if stdout supports colors (delegates to centralized ui module)
fn use_colors() -> bool {
    stdout_supports_color()
}

/// Create a cell with optional color
fn cell(text: &str) -> Cell {
    Cell::new(text)
}

/// Create an ID cell (cyan, bold when colors enabled)
fn id_cell(text: &str) -> Cell {
    if use_colors() {
        Cell::new(text)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold)
    } else {
        Cell::new(text)
    }
}

/// Create a status cell with semantic color
fn status_cell(status: &str) -> Cell {
    if use_colors() {
        let color = match status {
            "draft" | "proposed" | "queue" => Color::Yellow,
            "normative" | "accepted" | "active" | "done" => Color::Green,
            "deprecated" | "superseded" | "cancelled" => Color::DarkGrey,
            _ => Color::White,
        };
        Cell::new(status).fg(color)
    } else {
        Cell::new(status)
    }
}

/// Create a header cell (bold when colors enabled)
fn header_cell(text: &str) -> Cell {
    if use_colors() {
        Cell::new(text).add_attribute(Attribute::Bold)
    } else {
        Cell::new(text)
    }
}

/// List artifacts
pub fn list(
    config: &Config,
    target: ListTarget,
    filter: Option<&str>,
) -> anyhow::Result<Vec<Diagnostic>> {
    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };

    match target {
        ListTarget::Rfc => list_rfcs(&index, filter),
        ListTarget::Clause => list_clauses(&index, filter),
        ListTarget::Adr => list_adrs(&index, filter),
        ListTarget::Work => list_work_items(&index, filter),
    }

    Ok(vec![])
}

fn list_rfcs(index: &crate::model::ProjectIndex, filter: Option<&str>) {
    let mut rfcs: Vec<_> = index.rfcs.iter().collect();

    // Filter by status or phase if provided
    if let Some(f) = filter {
        rfcs.retain(|r| {
            r.rfc.status.as_ref() == f || r.rfc.phase.as_ref() == f || r.rfc.rfc_id.contains(f)
        });
    }

    rfcs.sort_by(|a, b| a.rfc.rfc_id.cmp(&b.rfc.rfc_id));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            header_cell("RFC"),
            header_cell("Version"),
            header_cell("Status"),
            header_cell("Phase"),
            header_cell("Title"),
        ]);

    for rfc in rfcs {
        // Check if RFC has been amended
        let rfc_id_display = if crate::signature::is_rfc_amended(rfc) {
            format!("{}*", rfc.rfc.rfc_id)
        } else {
            rfc.rfc.rfc_id.clone()
        };

        table.add_row(vec![
            id_cell(&rfc_id_display),
            cell(&rfc.rfc.version),
            status_cell(rfc.rfc.status.as_ref()),
            status_cell(rfc.rfc.phase.as_ref()),
            cell(&rfc.rfc.title),
        ]);
    }

    println!("{table}");
}

fn list_clauses(index: &crate::model::ProjectIndex, filter: Option<&str>) {
    let mut clauses: Vec<_> = index
        .iter_clauses()
        .map(|(rfc, clause)| (rfc.rfc.rfc_id.clone(), clause))
        .collect();

    // Filter by RFC ID if provided
    if let Some(f) = filter {
        clauses.retain(|(rfc_id, c)| {
            rfc_id == f || c.spec.clause_id.contains(f) || c.spec.status.as_ref() == f
        });
    }

    clauses.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.spec.clause_id.cmp(&b.1.spec.clause_id))
    });

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            header_cell("Clause"),
            header_cell("RFC"),
            header_cell("Kind"),
            header_cell("Status"),
            header_cell("Title"),
        ]);

    for (rfc_id, clause) in clauses {
        table.add_row(vec![
            id_cell(&clause.spec.clause_id),
            id_cell(&rfc_id),
            cell(clause.spec.kind.as_ref()),
            status_cell(clause.spec.status.as_ref()),
            cell(&clause.spec.title),
        ]);
    }

    println!("{table}");
}

fn list_adrs(index: &crate::model::ProjectIndex, filter: Option<&str>) {
    let mut adrs: Vec<_> = index.adrs.iter().collect();

    // Filter by status if provided
    if let Some(f) = filter {
        adrs.retain(|a| a.meta().status.as_ref() == f || a.meta().id.contains(f));
    }

    adrs.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            header_cell("ADR"),
            header_cell("Status"),
            header_cell("Date"),
            header_cell("Title"),
        ]);

    for adr in adrs {
        table.add_row(vec![
            id_cell(&adr.meta().id),
            status_cell(adr.meta().status.as_ref()),
            cell(&adr.meta().date),
            cell(&adr.meta().title),
        ]);
    }

    println!("{table}");
}

fn list_work_items(index: &crate::model::ProjectIndex, filter: Option<&str>) {
    let mut items: Vec<_> = index.work_items.iter().collect();

    // Default filter is "pending" (queue + active)
    let effective_filter = filter.unwrap_or("pending");

    match effective_filter {
        "all" => {}
        "pending" => {
            items.retain(|i| {
                i.meta().status == WorkItemStatus::Queue
                    || i.meta().status == WorkItemStatus::Active
            });
        }
        "queue" => items.retain(|i| i.meta().status == WorkItemStatus::Queue),
        "active" => items.retain(|i| i.meta().status == WorkItemStatus::Active),
        "done" => items.retain(|i| i.meta().status == WorkItemStatus::Done),
        "cancelled" => items.retain(|i| i.meta().status == WorkItemStatus::Cancelled),
        other => {
            items.retain(|i| i.meta().status.as_ref() == other || i.meta().id.contains(other));
        }
    }

    items.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            header_cell("ID"),
            header_cell("Status"),
            header_cell("Title"),
        ]);

    for item in items {
        table.add_row(vec![
            id_cell(&item.meta().id),
            status_cell(item.meta().status.as_ref()),
            cell(&item.meta().title),
        ]);
    }

    println!("{table}");
}
