#![allow(dead_code)] // Planner helpers are wired into the loop command surface in the next slice.

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
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::loop_state::LoopWorkItemStatus;
    use crate::model::{
        WorkItemContent, WorkItemEntry, WorkItemMeta, WorkItemSpec, WorkItemStatus,
        WorkItemVerification,
    };
    use std::path::PathBuf;

    fn work_item(id: &str, status: WorkItemStatus, depends_on: &[&str]) -> WorkItemEntry {
        WorkItemEntry {
            spec: WorkItemSpec {
                govctl: WorkItemMeta {
                    schema: 2,
                    id: id.to_string(),
                    title: id.to_string(),
                    status,
                    created: None,
                    started: None,
                    completed: None,
                    refs: vec![],
                    depends_on: depends_on.iter().map(|id| (*id).to_string()).collect(),
                    tags: vec![],
                },
                content: WorkItemContent::default(),
                verification: WorkItemVerification::default(),
            },
            path: PathBuf::from(format!("{id}.toml")),
        }
    }

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn assert_diagnostic_code<T>(
        result: anyhow::Result<T>,
        code: DiagnosticCode,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Err(err) = result else {
            return Err(format!("expected diagnostic {}", code.code()).into());
        };
        let Some(diagnostic) = err.downcast_ref::<Diagnostic>() else {
            return Err(format!("expected Diagnostic error, got: {err}").into());
        };
        assert_eq!(diagnostic.code, code);
        assert!(
            diagnostic.message.contains(text),
            "diagnostic should contain '{text}', got: {}",
            diagnostic.message
        );
        Ok(())
    }

    #[test]
    fn test_loop_plan_single_work_item() -> Result<(), Box<dyn std::error::Error>> {
        let root = "WI-2026-05-31-010";
        let plan = build_loop_plan(
            "loop-single",
            &ids(&[root]),
            &[work_item(root, WorkItemStatus::Queue, &[])],
        )?;

        assert_eq!(plan.topological_order, ids(&[root]));
        assert_eq!(plan.state.loop_meta.root_work_items, ids(&[root]));
        assert_eq!(plan.state.loop_meta.work_items, ids(&[root]));
        assert_eq!(plan.state.dependencies[root], Vec::<String>::new());
        assert_eq!(plan.state.items[root].status, LoopWorkItemStatus::Pending);
        Ok(())
    }

    #[test]
    fn test_loop_plan_resolves_dependency_closure_and_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = "WI-2026-05-31-014";
        let dependency_a = "WI-2026-05-31-011";
        let dependency_b = "WI-2026-05-31-012";
        let transitive = "WI-2026-05-31-013";
        let plan = build_loop_plan(
            "loop-multi",
            &ids(&[root]),
            &[
                work_item(root, WorkItemStatus::Queue, &[dependency_a, dependency_b]),
                work_item(dependency_a, WorkItemStatus::Queue, &[transitive]),
                work_item(dependency_b, WorkItemStatus::Queue, &[]),
                work_item(transitive, WorkItemStatus::Queue, &[]),
            ],
        )?;

        assert_eq!(
            plan.state.loop_meta.work_items,
            ids(&[dependency_a, dependency_b, transitive, root])
        );
        assert_eq!(
            plan.topological_order,
            ids(&[dependency_b, transitive, dependency_a, root])
        );
        assert_eq!(
            plan.state.dependencies[root],
            ids(&[dependency_a, dependency_b])
        );
        assert_eq!(plan.state.dependencies[dependency_a], ids(&[transitive]));
        Ok(())
    }

    #[test]
    fn test_loop_plan_rejects_missing_dependency() -> Result<(), Box<dyn std::error::Error>> {
        let root = "WI-2026-05-31-020";
        let missing = "WI-2026-05-31-999";

        assert_diagnostic_code(
            build_loop_plan(
                "loop-missing",
                &ids(&[root]),
                &[work_item(root, WorkItemStatus::Queue, &[missing])],
            ),
            DiagnosticCode::E1205LoopDependencyNotFound,
            missing,
        )
    }

    #[test]
    fn test_loop_plan_rejects_dependency_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let first = "WI-2026-05-31-030";
        let second = "WI-2026-05-31-031";

        assert_diagnostic_code(
            build_loop_plan(
                "loop-cycle",
                &ids(&[first]),
                &[
                    work_item(first, WorkItemStatus::Queue, &[second]),
                    work_item(second, WorkItemStatus::Queue, &[first]),
                ],
            ),
            DiagnosticCode::E1206LoopDependencyCycle,
            first,
        )
    }

    #[test]
    fn test_loop_plan_propagates_blocked_outcomes() -> Result<(), Box<dyn std::error::Error>> {
        let root = "WI-2026-05-31-043";
        let middle = "WI-2026-05-31-042";
        let failed = "WI-2026-05-31-041";
        let mut plan = build_loop_plan(
            "loop-blocked",
            &ids(&[root]),
            &[
                work_item(root, WorkItemStatus::Queue, &[middle]),
                work_item(middle, WorkItemStatus::Queue, &[failed]),
                work_item(failed, WorkItemStatus::Queue, &[]),
            ],
        )?;

        plan.state
            .set_item_status(failed, LoopWorkItemStatus::Failed)?;
        let blocked = propagate_blocked_outcomes(&mut plan.state)?;

        assert_eq!(blocked, ids(&[middle, root]));
        assert_eq!(plan.state.items[middle].status, LoopWorkItemStatus::Blocked);
        assert_eq!(plan.state.items[root].status, LoopWorkItemStatus::Blocked);
        Ok(())
    }

    #[test]
    fn test_loop_plan_marks_dependents_blocked_for_pre_existing_cancelled_dependency()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = "WI-2026-05-31-052";
        let done_middle = "WI-2026-05-31-051";
        let cancelled = "WI-2026-05-31-050";
        let plan = build_loop_plan(
            "loop-cancelled",
            &ids(&[root]),
            &[
                work_item(root, WorkItemStatus::Queue, &[done_middle]),
                work_item(done_middle, WorkItemStatus::Done, &[cancelled]),
                work_item(cancelled, WorkItemStatus::Cancelled, &[]),
            ],
        )?;

        assert_eq!(
            plan.state.items[cancelled].status,
            LoopWorkItemStatus::Cancelled
        );
        assert_eq!(
            plan.state.items[done_middle].status,
            LoopWorkItemStatus::Blocked
        );
        assert_eq!(plan.state.items[root].status, LoopWorkItemStatus::Blocked);
        assert_eq!(plan.topological_order, ids(&[cancelled, done_middle, root]));
        Ok(())
    }
}
