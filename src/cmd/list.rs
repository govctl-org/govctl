//! List command implementation.

mod output;

use crate::ListTarget;
use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::model::WorkItemStatus;
use crate::parse::load_guards_with_warnings;
use output::{output_list, truncate_chars};
use serde::Serialize;

/// List artifacts
pub fn list(
    config: &Config,
    target: ListTarget,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) -> anyhow::Result<Vec<Diagnostic>> {
    if target == ListTarget::Guard {
        let result = load_guards_with_warnings(config).map_err(anyhow::Error::from)?;
        list_guards(&result.items, filter, limit, output, tags);
        return Ok(result.warnings);
    }

    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };

    match target {
        ListTarget::Rfc => list_rfcs(&index, filter, limit, output, tags),
        ListTarget::Clause => list_clauses(&index, filter, limit, output, tags),
        ListTarget::Adr => list_adrs(&index, filter, limit, output, tags),
        ListTarget::Work => list_work_items(&index, filter, limit, output, tags),
        ListTarget::Guard => unreachable!("handled above"),
    }

    Ok(vec![])
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
    tags: &[String],
) {
    let mut rfcs: Vec<_> = index.rfcs.iter().collect();

    // Filter by status or phase if provided
    if let Some(f) = filter {
        rfcs.retain(|r| {
            r.rfc.status.as_ref() == f || r.rfc.phase.as_ref() == f || r.rfc.rfc_id.contains(f)
        });
    }

    // Filter by tags (artifact must have ALL specified tags)
    if !tags.is_empty() {
        rfcs.retain(|r| tags.iter().all(|t| r.rfc.tags.contains(t)));
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
    tags: &[String],
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

    // Filter by tags (clause must have ALL specified tags)
    if !tags.is_empty() {
        clauses.retain(|(_, c)| tags.iter().all(|t| c.spec.tags.contains(t)));
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
    tags: &[String],
) {
    let mut adrs: Vec<_> = index.adrs.iter().collect();

    // Filter by status if provided
    if let Some(f) = filter {
        adrs.retain(|a| a.meta().status.as_ref() == f || a.meta().id.contains(f));
    }

    // Filter by tags (artifact must have ALL specified tags)
    if !tags.is_empty() {
        adrs.retain(|a| tags.iter().all(|t| a.meta().tags.contains(t)));
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

/// Serializable guard summary for JSON output
#[derive(Serialize)]
struct GuardSummary {
    id: String,
    title: String,
    command: String,
}

fn list_guards(
    guards: &[crate::model::GuardEntry],
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) {
    let mut items: Vec<_> = guards.iter().collect();

    if let Some(f) = filter {
        items.retain(|g| g.meta().id.contains(f) || g.meta().title.contains(f));
    }

    // Filter by tags (artifact must have ALL specified tags)
    if !tags.is_empty() {
        items.retain(|g| tags.iter().all(|t| g.meta().tags.contains(t)));
    }

    items.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));

    if let Some(n) = limit {
        items.truncate(n);
    }

    let summaries: Vec<GuardSummary> = items
        .iter()
        .map(|g| GuardSummary {
            id: g.meta().id.clone(),
            title: g.meta().title.clone(),
            command: g.spec.check.command.clone(),
        })
        .collect();

    output_list(&summaries, &["Guard", "Title", "Command"], output, |s| {
        let cmd_display = truncate_chars(&s.command, 50);
        vec![s.id.clone(), s.title.clone(), cmd_display]
    });
}

fn list_work_items(
    index: &crate::model::ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
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

    // Filter by tags (artifact must have ALL specified tags)
    if !tags.is_empty() {
        items.retain(|i| tags.iter().all(|t| i.meta().tags.contains(t)));
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
