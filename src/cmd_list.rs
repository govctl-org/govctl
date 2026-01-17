//! List command implementation.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::model::WorkItemStatus;
use crate::ListTarget;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

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
        .set_header(vec!["RFC", "Version", "Status", "Phase", "Title"]);

    for rfc in rfcs {
        table.add_row(vec![
            rfc.rfc.rfc_id.as_str(),
            rfc.rfc.version.as_str(),
            rfc.rfc.status.as_ref(),
            rfc.rfc.phase.as_ref(),
            rfc.rfc.title.as_str(),
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
        .set_header(vec!["Clause", "RFC", "Kind", "Status", "Title"]);

    for (rfc_id, clause) in clauses {
        table.add_row(vec![
            clause.spec.clause_id.as_str(),
            rfc_id.as_str(),
            clause.spec.kind.as_ref(),
            clause.spec.status.as_ref(),
            clause.spec.title.as_str(),
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
        .set_header(vec!["ADR", "Status", "Date", "Title"]);

    for adr in adrs {
        table.add_row(vec![
            adr.meta().id.as_str(),
            adr.meta().status.as_ref(),
            adr.meta().date.as_str(),
            adr.meta().title.as_str(),
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
        .set_header(vec!["ID", "Status", "Title"]);

    for item in items {
        table.add_row(vec![
            item.meta().id.as_str(),
            item.meta().status.as_ref(),
            item.meta().title.as_str(),
        ]);
    }

    println!("{table}");
}
