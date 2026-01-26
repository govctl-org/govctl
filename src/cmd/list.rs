//! List command implementation.

use crate::ListTarget;
use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::model::WorkItemStatus;
use crate::ui::stdout_supports_color;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table, presets::UTF8_FULL};
use serde::Serialize;

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
    limit: Option<usize>,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };

    match target {
        ListTarget::Rfc => list_rfcs(&index, filter, limit, output),
        ListTarget::Clause => list_clauses(&index, filter, limit, output),
        ListTarget::Adr => list_adrs(&index, filter, limit, output),
        ListTarget::Work => list_work_items(&index, filter, limit, output),
    }

    Ok(vec![])
}

/// Output a list of items in the specified format
fn output_list<T: Serialize>(
    items: &[T],
    headers: &[&str],
    format: OutputFormat,
    to_row: impl Fn(&T) -> Vec<String>,
) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string())
            );
        }
        OutputFormat::Plain => {
            for item in items {
                let row = to_row(item);
                // Plain output: tab-separated values
                println!("{}", row.join("\t"));
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(headers.iter().map(|h| header_cell(h)).collect::<Vec<_>>());

            for item in items {
                let row = to_row(item);
                table.add_row(
                    row.iter()
                        .enumerate()
                        .map(|(i, v)| {
                            // First column is ID (cyan), status columns get semantic colors
                            if i == 0 {
                                id_cell(v)
                            } else if headers
                                .get(i)
                                .is_some_and(|h| *h == "Status" || *h == "Phase")
                            {
                                status_cell(v)
                            } else {
                                cell(v)
                            }
                        })
                        .collect::<Vec<_>>(),
                );
            }

            println!("{table}");
        }
    }
}

/// Serializable RFC summary for JSON output
#[derive(Serialize)]
struct RfcSummary {
    id: String,
    version: String,
    status: String,
    phase: String,
    title: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    amended: bool,
}

fn list_rfcs(
    index: &crate::model::ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
) {
    let mut rfcs: Vec<_> = index.rfcs.iter().collect();

    // Filter by status or phase if provided
    if let Some(f) = filter {
        rfcs.retain(|r| {
            r.rfc.status.as_ref() == f || r.rfc.phase.as_ref() == f || r.rfc.rfc_id.contains(f)
        });
    }

    rfcs.sort_by(|a, b| a.rfc.rfc_id.cmp(&b.rfc.rfc_id));

    // Apply limit if specified
    if let Some(n) = limit {
        rfcs.truncate(n);
    }

    // Convert to summaries
    let summaries: Vec<RfcSummary> = rfcs
        .iter()
        .map(|rfc| {
            let amended = crate::signature::is_rfc_amended(rfc);
            RfcSummary {
                id: if amended {
                    format!("{}*", rfc.rfc.rfc_id)
                } else {
                    rfc.rfc.rfc_id.clone()
                },
                version: rfc.rfc.version.clone(),
                status: rfc.rfc.status.as_ref().to_string(),
                phase: rfc.rfc.phase.as_ref().to_string(),
                title: rfc.rfc.title.clone(),
                amended,
            }
        })
        .collect();

    output_list(
        &summaries,
        &["RFC", "Version", "Status", "Phase", "Title"],
        output,
        |s| {
            vec![
                s.id.clone(),
                s.version.clone(),
                s.status.clone(),
                s.phase.clone(),
                s.title.clone(),
            ]
        },
    );
}

/// Serializable clause summary for JSON output
#[derive(Serialize)]
struct ClauseSummary {
    id: String,
    rfc_id: String,
    kind: String,
    status: String,
    title: String,
}

fn list_clauses(
    index: &crate::model::ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
) {
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

    // Apply limit if specified
    if let Some(n) = limit {
        clauses.truncate(n);
    }

    // Convert to summaries
    let summaries: Vec<ClauseSummary> = clauses
        .iter()
        .map(|(rfc_id, clause)| ClauseSummary {
            id: clause.spec.clause_id.clone(),
            rfc_id: rfc_id.clone(),
            kind: clause.spec.kind.as_ref().to_string(),
            status: clause.spec.status.as_ref().to_string(),
            title: clause.spec.title.clone(),
        })
        .collect();

    output_list(
        &summaries,
        &["Clause", "RFC", "Kind", "Status", "Title"],
        output,
        |s| {
            vec![
                s.id.clone(),
                s.rfc_id.clone(),
                s.kind.clone(),
                s.status.clone(),
                s.title.clone(),
            ]
        },
    );
}

/// Serializable ADR summary for JSON output
#[derive(Serialize)]
struct AdrSummary {
    id: String,
    status: String,
    date: String,
    title: String,
}

fn list_adrs(
    index: &crate::model::ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
) {
    let mut adrs: Vec<_> = index.adrs.iter().collect();

    // Filter by status if provided
    if let Some(f) = filter {
        adrs.retain(|a| a.meta().status.as_ref() == f || a.meta().id.contains(f));
    }

    adrs.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));

    // Apply limit if specified
    if let Some(n) = limit {
        adrs.truncate(n);
    }

    // Convert to summaries
    let summaries: Vec<AdrSummary> = adrs
        .iter()
        .map(|adr| AdrSummary {
            id: adr.meta().id.clone(),
            status: adr.meta().status.as_ref().to_string(),
            date: adr.meta().date.clone(),
            title: adr.meta().title.clone(),
        })
        .collect();

    output_list(
        &summaries,
        &["ADR", "Status", "Date", "Title"],
        output,
        |s| {
            vec![
                s.id.clone(),
                s.status.clone(),
                s.date.clone(),
                s.title.clone(),
            ]
        },
    );
}

/// Serializable work item summary for JSON output
#[derive(Serialize)]
struct WorkItemSummary {
    id: String,
    status: String,
    title: String,
}

fn list_work_items(
    index: &crate::model::ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
) {
    let mut items: Vec<_> = index.work_items.iter().collect();

    // Filter by status or ID if provided
    if let Some(f) = filter {
        match f {
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
    }

    items.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));

    // Apply limit if specified
    if let Some(n) = limit {
        items.truncate(n);
    }

    // Convert to summaries
    let summaries: Vec<WorkItemSummary> = items
        .iter()
        .map(|item| WorkItemSummary {
            id: item.meta().id.clone(),
            status: item.meta().status.as_ref().to_string(),
            title: item.meta().title.clone(),
        })
        .collect();

    output_list(&summaries, &["ID", "Status", "Title"], output, |s| {
        vec![s.id.clone(), s.status.clone(), s.title.clone()]
    });
}
