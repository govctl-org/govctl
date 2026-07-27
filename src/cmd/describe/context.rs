use crate::cmd::loop_cmd;
use crate::config::Config;
use crate::diagnostic::Diagnostics;
use crate::load::load_project_with_warnings;
use crate::loop_state::{LoopLifecycleState, LoopState};
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
use serde::Serialize;

#[derive(Serialize)]
pub struct ProjectState {
    pub counts: ProjectCounts,
    pub rfcs: Vec<RfcState>,
    pub adrs: Vec<AdrState>,
    pub work_items: Vec<WorkItemState>,
    pub loops: Vec<LoopStateInfo>,
}

#[derive(Serialize)]
pub struct ProjectCounts {
    pub rfcs: RfcCounts,
    pub adrs: AdrCounts,
    pub work_items: WorkItemCounts,
    pub loops: LoopCounts,
}

#[derive(Default, Serialize)]
pub struct RfcCounts {
    pub total: usize,
    pub draft: usize,
    pub normative: usize,
    pub deprecated: usize,
    pub spec: usize,
    #[serde(rename = "impl")]
    pub impl_phase: usize,
    pub test: usize,
    pub stable: usize,
}

#[derive(Default, Serialize)]
pub struct AdrCounts {
    pub total: usize,
    pub proposed: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub superseded: usize,
}

#[derive(Default, Serialize)]
pub struct WorkItemCounts {
    pub total: usize,
    pub queue: usize,
    pub active: usize,
    pub done: usize,
    pub cancelled: usize,
}

#[derive(Default, Serialize)]
pub struct LoopCounts {
    pub total: usize,
    pub pending: usize,
    pub active: usize,
    pub paused: usize,
    pub completed: usize,
    pub failed: usize,
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
pub struct LoopStateInfo {
    pub id: String,
    pub state: String,
    pub next_action: String,
    pub work: Vec<String>,
}

pub(super) struct DescribeContext {
    pub(super) project_state: ProjectState,
    pub(super) suggested_actions: Vec<String>,
}

pub(super) fn load_context(config: &Config) -> Result<DescribeContext, Diagnostics> {
    let load_result = load_project_with_warnings(config)?;
    if !load_result.warnings.is_empty() {
        return Err(load_result.warnings);
    }
    let index = load_result.index;
    let loop_states = loop_cmd::load_loop_states(config).map_err(|diagnostic| vec![diagnostic])?;

    // [[RFC-0002:C-DESCRIBE-COMMAND]] keeps full counts but enumerates only
    // state that can still affect current governance work.
    let mut rfc_counts = RfcCounts::default();
    let mut rfcs: Vec<_> = index
        .rfcs
        .iter()
        .filter_map(|entry| {
            rfc_counts.total += 1;
            match entry.rfc.status {
                RfcStatus::Draft => rfc_counts.draft += 1,
                RfcStatus::Normative => rfc_counts.normative += 1,
                RfcStatus::Deprecated => rfc_counts.deprecated += 1,
            }
            match entry.rfc.phase {
                RfcPhase::Spec => rfc_counts.spec += 1,
                RfcPhase::Impl => rfc_counts.impl_phase += 1,
                RfcPhase::Test => rfc_counts.test += 1,
                RfcPhase::Stable => rfc_counts.stable += 1,
            }

            let actionable = entry.rfc.status == RfcStatus::Draft
                || (entry.rfc.status == RfcStatus::Normative
                    && entry.rfc.phase != RfcPhase::Stable);
            actionable.then(|| RfcState {
                id: entry.rfc.rfc_id.clone(),
                title: entry.rfc.title.clone(),
                status: entry.rfc.status.as_ref().to_string(),
                phase: entry.rfc.phase.as_ref().to_string(),
            })
        })
        .collect();
    rfcs.sort_by(|left, right| left.id.cmp(&right.id));

    let mut adr_counts = AdrCounts::default();
    let mut adrs: Vec<_> = index
        .adrs
        .iter()
        .filter_map(|entry| {
            adr_counts.total += 1;
            match entry.meta().status {
                AdrStatus::Proposed => adr_counts.proposed += 1,
                AdrStatus::Accepted => adr_counts.accepted += 1,
                AdrStatus::Rejected => adr_counts.rejected += 1,
                AdrStatus::Superseded => adr_counts.superseded += 1,
            }

            (entry.meta().status == AdrStatus::Proposed).then(|| AdrState {
                id: entry.meta().id.clone(),
                title: entry.meta().title.clone(),
                status: entry.meta().status.as_ref().to_string(),
            })
        })
        .collect();
    adrs.sort_by(|left, right| left.id.cmp(&right.id));

    let mut work_item_counts = WorkItemCounts::default();
    let mut work_items: Vec<_> = index
        .work_items
        .iter()
        .filter_map(|entry| {
            work_item_counts.total += 1;
            match entry.meta().status {
                WorkItemStatus::Queue => work_item_counts.queue += 1,
                WorkItemStatus::Active => work_item_counts.active += 1,
                WorkItemStatus::Done => work_item_counts.done += 1,
                WorkItemStatus::Cancelled => work_item_counts.cancelled += 1,
            }

            matches!(
                entry.meta().status,
                WorkItemStatus::Queue | WorkItemStatus::Active
            )
            .then(|| WorkItemState {
                id: entry.meta().id.clone(),
                title: entry.meta().title.clone(),
                status: entry.meta().status.as_ref().to_string(),
            })
        })
        .collect();
    work_items.sort_by(|left, right| left.id.cmp(&right.id));

    let loop_counts = count_loops(&loop_states);
    let mut loops: Vec<_> = loop_states
        .iter()
        .filter(|state| is_non_terminal_loop(state.loop_meta.state))
        .map(|state| LoopStateInfo {
            id: state.loop_meta.id.clone(),
            state: state.loop_meta.state.as_str().to_string(),
            next_action: state.loop_meta.next_action.as_str().to_string(),
            work: state.loop_meta.work.clone(),
        })
        .collect();
    loops.sort_by(|left, right| left.id.cmp(&right.id));

    let project_state = ProjectState {
        counts: ProjectCounts {
            rfcs: rfc_counts,
            adrs: adr_counts,
            work_items: work_item_counts,
            loops: loop_counts,
        },
        rfcs,
        adrs,
        work_items,
        loops,
    };
    let suggested_actions = generate_suggestions(&project_state);

    Ok(DescribeContext {
        project_state,
        suggested_actions,
    })
}

fn count_loops(states: &[LoopState]) -> LoopCounts {
    let mut counts = LoopCounts::default();
    for state in states {
        counts.total += 1;
        match state.loop_meta.state {
            LoopLifecycleState::Pending => counts.pending += 1,
            LoopLifecycleState::Active => counts.active += 1,
            LoopLifecycleState::Paused => counts.paused += 1,
            LoopLifecycleState::Completed => counts.completed += 1,
            LoopLifecycleState::Failed => counts.failed += 1,
        }
    }
    counts
}

fn is_non_terminal_loop(state: LoopLifecycleState) -> bool {
    matches!(
        state,
        LoopLifecycleState::Pending | LoopLifecycleState::Active | LoopLifecycleState::Paused
    )
}

fn generate_suggestions(state: &ProjectState) -> Vec<String> {
    state
        .rfcs
        .iter()
        .map(|rfc| format!("govctl rfc show {}", rfc.id))
        .chain(
            state
                .adrs
                .iter()
                .map(|adr| format!("govctl adr show {}", adr.id)),
        )
        .chain(
            state
                .work_items
                .iter()
                .map(|work_item| format!("govctl work show {}", work_item.id)),
        )
        .chain(
            state
                .loops
                .iter()
                .map(|loop_state| format!("govctl loop resume {}", loop_state.id)),
        )
        .collect()
}
