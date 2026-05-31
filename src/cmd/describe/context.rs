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
            suggestions.push(SuggestedAction {
                command: format!("govctl rfc finalize {} normative", rfc.id),
                reason: format!(
                    "{} is in draft status. If the spec is complete, finalize it to make it binding.",
                    rfc.id
                ),
                priority: "medium".to_string(),
            });
        }

        match (rfc.status.as_str(), rfc.phase.as_str()) {
            ("normative", "spec") => {
                suggestions.push(SuggestedAction {
                    command: format!("govctl rfc advance {} impl", rfc.id),
                    reason: format!(
                        "{} is normative but still in spec phase. Advance to impl when ready to implement.",
                        rfc.id
                    ),
                    priority: "high".to_string(),
                });
            }
            ("normative", "impl") => {
                suggestions.push(SuggestedAction {
                    command: format!("govctl rfc advance {} test", rfc.id),
                    reason: format!(
                        "{} is in impl phase. Advance to test when implementation is complete.",
                        rfc.id
                    ),
                    priority: "medium".to_string(),
                });
            }
            ("normative", "test") => {
                suggestions.push(SuggestedAction {
                    command: format!("govctl rfc advance {} stable", rfc.id),
                    reason: format!(
                        "{} is in test phase. Advance to stable when tests pass.",
                        rfc.id
                    ),
                    priority: "medium".to_string(),
                });
            }
            _ => {}
        }
    }

    for adr in adrs {
        if adr.status == "proposed" {
            suggestions.push(SuggestedAction {
                command: format!("govctl adr accept {}", adr.id),
                reason: format!(
                    "{} is proposed. Accept it if the decision is approved.",
                    adr.id
                ),
                priority: "medium".to_string(),
            });
        }
    }

    let active_count = work_items.iter().filter(|w| w.status == "active").count();
    let queue_count = work_items.iter().filter(|w| w.status == "queue").count();

    if active_count == 0 && queue_count > 0 {
        suggestions.push(SuggestedAction {
            command: "govctl work list queue".to_string(),
            reason: format!(
                "No active work items but {} in queue. Consider activating one.",
                queue_count
            ),
            priority: "high".to_string(),
        });
    }

    for work_item in work_items {
        if work_item.status == "active" {
            suggestions.push(SuggestedAction {
                command: format!("govctl work move {} done", work_item.id),
                reason: format!(
                    "{} is active. Mark it done when acceptance criteria are met.",
                    work_item.id
                ),
                priority: "low".to_string(),
            });
        }
    }

    suggestions
}
