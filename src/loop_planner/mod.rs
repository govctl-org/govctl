use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_state::{LoopState, LoopWorkItemStatus};
use crate::model::{WorkItemEntry, WorkItemStatus};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopPlan {
    pub state: LoopState,
    pub topological_order: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

pub fn build_loop_plan_from_config(
    config: &Config,
    loop_id: &str,
    root_work_items: &[String],
) -> anyhow::Result<LoopPlan> {
    let work_items = crate::parse::load_work_items(config)?;
    build_loop_plan(loop_id, root_work_items, &work_items)
}

pub fn replan_loop_state_from_config(
    config: &Config,
    existing: &LoopState,
    root_work_items: &[String],
) -> anyhow::Result<LoopPlan> {
    let work_items = crate::parse::load_work_items(config)?;
    replan_loop_state(existing, root_work_items, &work_items)
}

pub fn build_loop_plan(
    loop_id: &str,
    root_work_items: &[String],
    work_items: &[WorkItemEntry],
) -> anyhow::Result<LoopPlan> {
    if root_work_items.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            "Loop root work item set must not be empty",
            loop_id,
        )
        .into());
    }

    let by_id = work_items
        .iter()
        .map(|entry| (entry.meta().id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    let mut visit = HashMap::new();
    let mut stack = Vec::new();
    let mut closure = BTreeSet::new();

    for root in root_work_items {
        resolve_dependency_closure(root, loop_id, &by_id, &mut visit, &mut stack, &mut closure)?;
    }

    let resolved_work_items = closure.iter().cloned().collect::<Vec<_>>();
    let dependencies = dependency_table(&resolved_work_items, &by_id);
    let topological_order = deterministic_execution_order(loop_id, &dependencies)?;

    let mut state = LoopState::new(
        loop_id,
        root_work_items.to_vec(),
        resolved_work_items.clone(),
        dependencies,
    )?;
    for work_id in &resolved_work_items {
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

pub fn propagate_blocked_outcomes(state: &mut LoopState) -> anyhow::Result<Vec<String>> {
    propagate_blocked_outcomes_inner(state, false)
}

pub fn recompute_scope_mutation_blocked_outcomes(
    state: &mut LoopState,
) -> anyhow::Result<Vec<String>> {
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
) -> anyhow::Result<Vec<String>> {
    state.validate(Some(&state.loop_meta.id))?;
    let mut blocked = Vec::new();

    loop {
        let mut changed = false;
        let work_items = state.loop_meta.work_items.clone();
        for work_id in work_items {
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
    root_work_items: &[String],
    work_items: &[WorkItemEntry],
) -> anyhow::Result<LoopPlan> {
    let mut plan = build_loop_plan(&existing.loop_meta.id, root_work_items, work_items)?;
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

pub fn topological_order_for_state(state: &LoopState) -> anyhow::Result<Vec<String>> {
    deterministic_execution_order(&state.loop_meta.id, &state.dependencies)
}

fn resolve_dependency_closure(
    work_id: &str,
    loop_id: &str,
    by_id: &HashMap<&str, &WorkItemEntry>,
    visit: &mut HashMap<String, VisitState>,
    stack: &mut Vec<String>,
    closure: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    if visit.get(work_id) == Some(&VisitState::Visiting) {
        return Err(cycle_error(loop_id, work_id, stack));
    }
    if visit.get(work_id) == Some(&VisitState::Visited) {
        return Ok(());
    }

    let entry = by_id.get(work_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E1205LoopDependencyNotFound,
            format!("Loop dependency work item not found: {work_id}"),
            loop_id,
        )
    })?;

    visit.insert(work_id.to_string(), VisitState::Visiting);
    stack.push(work_id.to_string());

    let mut dependencies = entry.meta().depends_on.clone();
    dependencies.sort();
    for dependency in dependencies {
        resolve_dependency_closure(&dependency, loop_id, by_id, visit, stack, closure)?;
    }

    stack.pop();
    visit.insert(work_id.to_string(), VisitState::Visited);
    closure.insert(work_id.to_string());
    Ok(())
}

fn cycle_error(loop_id: &str, work_id: &str, stack: &[String]) -> anyhow::Error {
    let start = stack.iter().position(|id| id == work_id).unwrap_or(0);
    let mut cycle = stack[start..].to_vec();
    cycle.push(work_id.to_string());
    Diagnostic::new(
        DiagnosticCode::E1206LoopDependencyCycle,
        format!("cyclic loop dependency detected: {}", cycle.join(" -> ")),
        loop_id,
    )
    .into()
}

fn dependency_table(
    resolved_work_items: &[String],
    by_id: &HashMap<&str, &WorkItemEntry>,
) -> BTreeMap<String, Vec<String>> {
    resolved_work_items
        .iter()
        .map(|work_id| {
            let mut dependencies = by_id
                .get(work_id.as_str())
                .map(|entry| entry.meta().depends_on.clone())
                .unwrap_or_default();
            dependencies.sort();
            (work_id.clone(), dependencies)
        })
        .collect()
}

fn deterministic_execution_order(
    loop_id: &str,
    dependencies: &BTreeMap<String, Vec<String>>,
) -> anyhow::Result<Vec<String>> {
    let mut remaining = dependencies
        .iter()
        .map(|(work_id, deps)| {
            (
                work_id.clone(),
                deps.iter().cloned().collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut dependents = BTreeMap::<String, BTreeSet<String>>::new();
    for (work_id, deps) in dependencies {
        for dependency in deps {
            dependents
                .entry(dependency.clone())
                .or_default()
                .insert(work_id.clone());
        }
    }

    let mut ready = remaining
        .iter()
        .filter_map(|(work_id, deps)| deps.is_empty().then_some(work_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::new();

    while let Some(work_id) = ready.iter().next().cloned() {
        ready.remove(&work_id);
        order.push(work_id.clone());

        if let Some(next_items) = dependents.get(&work_id) {
            for dependent in next_items {
                if let Some(deps) = remaining.get_mut(dependent) {
                    deps.remove(&work_id);
                    if deps.is_empty() {
                        ready.insert(dependent.clone());
                    }
                }
            }
        }
    }

    if order.len() != dependencies.len() {
        return Err(Diagnostic::new(
            DiagnosticCode::E1206LoopDependencyCycle,
            "cyclic loop dependency detected while ordering work items",
            loop_id,
        )
        .into());
    }

    Ok(order)
}

#[cfg(test)]
mod tests;
