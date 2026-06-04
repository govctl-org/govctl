use crate::config::Config;
use crate::load::load_project;
use serde::Serialize;

#[derive(Serialize)]
pub struct ProjectState {
    pub rfcs: Vec<RfcState>,
    pub adrs: Vec<AdrState>,
    pub work_items: Vec<WorkItemState>,
}

#[derive(Serialize)]
pub struct RfcState {
    pub id: String,
    pub title: String,
    pub status: String,
    pub phase: String,
}

#[derive(Serialize)]
pub struct AdrState {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct WorkItemState {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct SuggestedAction {
    pub command: String,
    pub reason: String,
    pub priority: String,
}

pub(super) struct DescribeContext {
    pub(super) project_state: ProjectState,
    pub(super) suggested_actions: Vec<SuggestedAction>,
}

fn suggested_action(
    command: impl Into<String>,
    reason: impl Into<String>,
    priority: &str,
) -> SuggestedAction {
    SuggestedAction {
        command: command.into(),
        reason: reason.into(),
        priority: priority.to_string(),
    }
}

pub(super) fn load_context(config: &Config) -> Option<DescribeContext> {
    let index = load_project(config).ok()?;

    let rfcs: Vec<RfcState> = index
        .rfcs
        .iter()
        .map(|r| RfcState {
            id: r.rfc.rfc_id.clone(),
            title: r.rfc.title.clone(),
            status: r.rfc.status.as_ref().to_string(),
            phase: r.rfc.phase.as_ref().to_string(),
        })
        .collect();

    let adrs: Vec<AdrState> = index
        .adrs
        .iter()
        .map(|a| AdrState {
            id: a.meta().id.clone(),
            title: a.meta().title.clone(),
            status: a.meta().status.as_ref().to_string(),
        })
        .collect();

    let work_items: Vec<WorkItemState> = index
        .work_items
        .iter()
        .map(|w| WorkItemState {
            id: w.meta().id.clone(),
            title: w.meta().title.clone(),
            status: w.meta().status.as_ref().to_string(),
        })
        .collect();

    let suggested_actions = generate_suggestions(&rfcs, &adrs, &work_items);
    let project_state = ProjectState {
        rfcs,
        adrs,
        work_items,
    };

    Some(DescribeContext {
        project_state,
        suggested_actions,
    })
}

/// Generate suggested actions based on project state
fn generate_suggestions(
    rfcs: &[RfcState],
    adrs: &[AdrState],
    work_items: &[WorkItemState],
) -> Vec<SuggestedAction> {
    let mut suggestions = Vec::new();

    for rfc in rfcs {
        if rfc.status == "draft" {
            suggestions.push(suggested_action(
                format!("govctl rfc finalize {} normative", rfc.id),
                format!(
                    "{} is in draft status. If the spec is complete, finalize it to make it binding.",
                    rfc.id
                ),
                "medium",
            ));
        }

        match (rfc.status.as_str(), rfc.phase.as_str()) {
            ("normative", "spec") => {
                suggestions.push(suggested_action(
                    format!("govctl rfc advance {} impl", rfc.id),
                    format!(
                        "{} is normative but still in spec phase. Advance to impl when ready to implement.",
                        rfc.id
                    ),
                    "high",
                ));
            }
            ("normative", "impl") => {
                suggestions.push(suggested_action(
                    format!("govctl rfc advance {} test", rfc.id),
                    format!(
                        "{} is in impl phase. Advance to test when implementation is complete.",
                        rfc.id
                    ),
                    "medium",
                ));
            }
            ("normative", "test") => {
                suggestions.push(suggested_action(
                    format!("govctl rfc advance {} stable", rfc.id),
                    format!(
                        "{} is in test phase. Advance to stable when tests pass.",
                        rfc.id
                    ),
                    "medium",
                ));
            }
            _ => {}
        }
    }

    for adr in adrs {
        if adr.status == "proposed" {
            suggestions.push(suggested_action(
                format!("govctl adr accept {}", adr.id),
                format!(
                    "{} is proposed. Accept it if the decision is approved.",
                    adr.id
                ),
                "medium",
            ));
        }
    }

    let active_count = work_items.iter().filter(|w| w.status == "active").count();
    let queue_count = work_items.iter().filter(|w| w.status == "queue").count();

    if active_count == 0 && queue_count > 0 {
        suggestions.push(suggested_action(
            "govctl work list queue",
            format!(
                "No active work items but {} in queue. Consider activating one.",
                queue_count
            ),
            "high",
        ));
    }

    for work_item in work_items {
        if work_item.status == "active" {
            suggestions.push(suggested_action(
                format!("govctl work move {} done", work_item.id),
                format!(
                    "{} is active. Mark it done when acceptance criteria are met.",
                    work_item.id
                ),
                "low",
            ));
        }
    }

    suggestions
}
