use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::WorkItemEntry;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

pub(super) fn resolve_dependency_closure(
    loop_id: &str,
    work: &[String],
    by_id: &HashMap<&str, &WorkItemEntry>,
) -> DiagnosticResult<Vec<String>> {
    let mut visit = HashMap::new();
    let mut stack = Vec::new();
    let mut closure = BTreeSet::new();

    for work_id in work {
        visit_dependency_closure(
            work_id,
            loop_id,
            by_id,
            &mut visit,
            &mut stack,
            &mut closure,
        )?;
    }

    Ok(closure.iter().cloned().collect())
}

fn visit_dependency_closure(
    work_id: &str,
    loop_id: &str,
    by_id: &HashMap<&str, &WorkItemEntry>,
    visit: &mut HashMap<String, VisitState>,
    stack: &mut Vec<String>,
    closure: &mut BTreeSet<String>,
) -> DiagnosticResult<()> {
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
        visit_dependency_closure(&dependency, loop_id, by_id, visit, stack, closure)?;
    }

    stack.pop();
    visit.insert(work_id.to_string(), VisitState::Visited);
    closure.insert(work_id.to_string());
    Ok(())
}

fn cycle_error(loop_id: &str, work_id: &str, stack: &[String]) -> Diagnostic {
    let start = stack.iter().position(|id| id == work_id).unwrap_or(0);
    let mut cycle = stack[start..].to_vec();
    cycle.push(work_id.to_string());
    Diagnostic::new(
        DiagnosticCode::E1206LoopDependencyCycle,
        format!("cyclic loop dependency detected: {}", cycle.join(" -> ")),
        loop_id,
    )
}

pub(super) fn dependency_table(
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

pub(super) fn deterministic_execution_order(
    loop_id: &str,
    dependencies: &BTreeMap<String, Vec<String>>,
) -> DiagnosticResult<Vec<String>> {
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
        ));
    }

    Ok(order)
}
