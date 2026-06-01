mod graph;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_state::{LoopState, LoopWorkItemStatus};
use crate::model::{WorkItemEntry, WorkItemStatus};
use graph::{dependency_table, deterministic_execution_order, resolve_dependency_closure};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopPlan {
    pub state: LoopState,
    pub topological_order: Vec<String>,
}

pub fn build_loop_plan_from_config(
    config: &Config,
    loop_id: &str,
    work: &[String],
) -> DiagnosticResult<LoopPlan> {
    let all_work_items = crate::parse::load_work_items(config)?;
    build_loop_plan(loop_id, work, &all_work_items)
}

pub fn replan_loop_state_from_config(
    config: &Config,
    existing: &LoopState,
    work: &[String],
) -> DiagnosticResult<LoopPlan> {
    let all_work_items = crate::parse::load_work_items(config)?;
    replan_loop_state(existing, work, &all_work_items)
}

pub fn build_loop_plan(
    loop_id: &str,
    work: &[String],
    all_work_items: &[WorkItemEntry],
) -> DiagnosticResult<LoopPlan> {
    if work.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            "Loop work item set must not be empty",
            loop_id,
        ));
    }

    let by_id = all_work_items
        .iter()
        .map(|entry| (entry.meta().id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    let resolved_work_ids = resolve_dependency_closure(loop_id, work, &by_id)?;
    let dependencies = dependency_table(&resolved_work_ids, &by_id);
    let topological_order = deterministic_execution_order(loop_id, &dependencies)?;

    let mut state = LoopState::new(
        loop_id,
        work.to_vec(),
        resolved_work_ids.clone(),
        dependencies,
    )?;
    for work_id in &resolved_work_ids {
        let entry = by_id.get(work_id.as_str()).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1205LoopDependencyNotFound,
                format!("Loop work item disappeared while planning: {work_id}"),
                loop_id,
            )
        })?;
        match entry.meta().status {
            WorkItemStatus::Done => state.set_item_status(work_id, LoopWorkItemStatus::Done)?,
            WorkItemStatus::Cancelled => {
                state.set_item_status(work_id, LoopWorkItemStatus::Cancelled)?;
            }
            WorkItemStatus::Queue | WorkItemStatus::Active => {}
        }
    }

    propagate_blocked_outcomes(&mut state)?;

    Ok(LoopPlan {
        state,
        topological_order,
    })
}

pub fn propagate_blocked_outcomes(state: &mut LoopState) -> DiagnosticResult<Vec<String>> {
    propagate_blocked_outcomes_inner(state, false)
}

pub fn recompute_scope_mutation_blocked_outcomes(
    state: &mut LoopState,
) -> DiagnosticResult<Vec<String>> {
    for item in state.items.values_mut() {
        if item.status == LoopWorkItemStatus::Blocked {
            item.status = LoopWorkItemStatus::Pending;
        }
    }
    propagate_blocked_outcomes_inner(state, true)
}

fn propagate_blocked_outcomes_inner(
    state: &mut LoopState,
    preserve_done: bool,
) -> DiagnosticResult<Vec<String>> {
    state.validate(Some(&state.loop_meta.id))?;
    let mut blocked = Vec::new();

    loop {
        let mut changed = false;
        let resolved_work_ids = state.loop_meta.resolved.clone();
        for work_id in resolved_work_ids {
            if matches!(
                state.items[work_id.as_str()].status,
                LoopWorkItemStatus::Failed
                    | LoopWorkItemStatus::Blocked
                    | LoopWorkItemStatus::Cancelled
            ) {
                continue;
            }
            if preserve_done && state.items[work_id.as_str()].status == LoopWorkItemStatus::Done {
                continue;
            }

            let dependencies = state.dependencies.get(&work_id).ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E1201LoopStateInvalid,
                    format!("missing dependency entry for work item: {work_id}"),
                    state.loop_meta.id.clone(),
                )
            })?;

            if dependencies.iter().any(|dependency| {
                matches!(
                    state.items[dependency.as_str()].status,
                    LoopWorkItemStatus::Failed
                        | LoopWorkItemStatus::Blocked
                        | LoopWorkItemStatus::Cancelled
                )
            }) {
                state.set_item_status(&work_id, LoopWorkItemStatus::Blocked)?;
                blocked.push(work_id);
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    Ok(blocked)
}

pub fn replan_loop_state(
    existing: &LoopState,
    work: &[String],
    all_work_items: &[WorkItemEntry],
) -> DiagnosticResult<LoopPlan> {
    let mut plan = build_loop_plan(&existing.loop_meta.id, work, all_work_items)?;
    plan.state.loop_meta.state = existing.loop_meta.state;

    for (work_id, item) in &mut plan.state.items {
        let Some(previous) = existing.items.get(work_id) else {
            continue;
        };
        item.round_count = previous.round_count;
        item.status = preserved_replan_status(previous.status, item.status);
    }

    recompute_scope_mutation_blocked_outcomes(&mut plan.state)?;
    Ok(plan)
}

fn preserved_replan_status(
    previous: LoopWorkItemStatus,
    current: LoopWorkItemStatus,
) -> LoopWorkItemStatus {
    match (previous, current) {
        (LoopWorkItemStatus::Done, _) => LoopWorkItemStatus::Done,
        (LoopWorkItemStatus::Failed, _) => LoopWorkItemStatus::Failed,
        (LoopWorkItemStatus::Cancelled, _) => LoopWorkItemStatus::Cancelled,
        (_, LoopWorkItemStatus::Cancelled) => LoopWorkItemStatus::Cancelled,
        (_, LoopWorkItemStatus::Done) => LoopWorkItemStatus::Done,
        (LoopWorkItemStatus::Blocked, _) => LoopWorkItemStatus::Pending,
        (LoopWorkItemStatus::Active, LoopWorkItemStatus::Pending) => LoopWorkItemStatus::Active,
        (LoopWorkItemStatus::Pending, LoopWorkItemStatus::Pending) => LoopWorkItemStatus::Pending,
        (_, LoopWorkItemStatus::Blocked) => LoopWorkItemStatus::Blocked,
        (_, LoopWorkItemStatus::Active) => current,
        (_, LoopWorkItemStatus::Failed) => current,
    }
}

pub fn topological_order_for_state(state: &LoopState) -> DiagnosticResult<Vec<String>> {
    deterministic_execution_order(&state.loop_meta.id, &state.dependencies)
}

#[cfg(test)]
mod tests;
