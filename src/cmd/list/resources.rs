use super::output::output_list;
use super::summaries::{AdrSummary, ClauseSummary, GuardSummary, RfcSummary, WorkItemSummary};
use crate::OutputFormat;
use crate::model::{GuardEntry, ProjectIndex, WorkItemStatus};

pub(super) fn list_rfcs(
    index: &ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) {
    let mut rfcs: Vec<_> = index.rfcs.iter().collect();

    if let Some(f) = filter {
        rfcs.retain(|r| {
            r.rfc.status.as_ref() == f || r.rfc.phase.as_ref() == f || r.rfc.rfc_id.contains(f)
        });
    }

    retain_by_tags(&mut rfcs, tags, |r| r.rfc.tags.as_slice());

    rfcs.sort_by(|a, b| a.rfc.rfc_id.cmp(&b.rfc.rfc_id));
    apply_limit(&mut rfcs, limit);

    let summaries = rfcs
        .iter()
        .map(|rfc| RfcSummary::from_entry(rfc))
        .collect::<Vec<_>>();

    output_list(
        &summaries,
        &["RFC", "Version", "Status", "Phase", "Title"],
        output,
        RfcSummary::row,
    );
}

pub(super) fn list_clauses(
    index: &ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) {
    let mut clauses: Vec<_> = index
        .iter_clauses()
        .map(|(rfc, clause)| (rfc.rfc.rfc_id.clone(), clause))
        .collect();

    if let Some(f) = filter {
        clauses.retain(|(rfc_id, c)| {
            rfc_id == f || c.spec.clause_id.contains(f) || c.spec.status.as_ref() == f
        });
    }

    retain_by_tags(&mut clauses, tags, |(_, c)| c.spec.tags.as_slice());

    clauses.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.spec.clause_id.cmp(&b.1.spec.clause_id))
    });
    apply_limit(&mut clauses, limit);

    let summaries = clauses
        .iter()
        .map(|(rfc_id, clause)| ClauseSummary::from_entry(rfc_id, clause))
        .collect::<Vec<_>>();

    output_list(
        &summaries,
        &["Clause", "RFC", "Kind", "Status", "Title"],
        output,
        ClauseSummary::row,
    );
}

pub(super) fn list_adrs(
    index: &ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) {
    let mut adrs: Vec<_> = index.adrs.iter().collect();

    if let Some(f) = filter {
        adrs.retain(|a| a.meta().status.as_ref() == f || a.meta().id.contains(f));
    }

    retain_by_tags(&mut adrs, tags, |a| a.meta().tags.as_slice());

    adrs.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));
    apply_limit(&mut adrs, limit);

    let summaries = adrs
        .iter()
        .map(|adr| AdrSummary::from_entry(adr))
        .collect::<Vec<_>>();

    output_list(
        &summaries,
        &["ADR", "Status", "Date", "Title"],
        output,
        AdrSummary::row,
    );
}

pub(super) fn list_guards(
    guards: &[GuardEntry],
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) {
    let mut items: Vec<_> = guards.iter().collect();

    if let Some(f) = filter {
        items.retain(|g| g.meta().id.contains(f) || g.meta().title.contains(f));
    }

    retain_by_tags(&mut items, tags, |g| g.meta().tags.as_slice());

    items.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));
    apply_limit(&mut items, limit);

    let summaries = items
        .iter()
        .map(|guard| GuardSummary::from_entry(guard))
        .collect::<Vec<_>>();

    output_list(
        &summaries,
        &["Guard", "Title", "Command"],
        output,
        GuardSummary::row,
    );
}

pub(super) fn list_work_items(
    index: &ProjectIndex,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) {
    let mut items: Vec<_> = index.work_items.iter().collect();

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

    retain_by_tags(&mut items, tags, |i| i.meta().tags.as_slice());

    items.sort_by(|a, b| a.meta().id.cmp(&b.meta().id));
    apply_limit(&mut items, limit);

    let summaries = items
        .iter()
        .map(|item| WorkItemSummary::from_entry(item))
        .collect::<Vec<_>>();

    output_list(
        &summaries,
        &["ID", "Status", "Title"],
        output,
        WorkItemSummary::row,
    );
}

fn apply_limit<T>(items: &mut Vec<T>, limit: Option<usize>) {
    if let Some(n) = limit {
        items.truncate(n);
    }
}

fn retain_by_tags<T, F>(items: &mut Vec<T>, required: &[String], mut tags_for: F)
where
    F: for<'a> FnMut(&'a T) -> &'a [String],
{
    if required.is_empty() {
        return;
    }
    items.retain(|item| {
        let item_tags = tags_for(item);
        required.iter().all(|tag| item_tags.contains(tag))
    });
}
