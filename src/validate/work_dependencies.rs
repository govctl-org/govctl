use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkDependencyVisit {
    Visiting,
    Visited,
}

pub fn is_work_item_id(value: &str) -> bool {
    Regex::new(r"^WI-\d{4}-\d{2}-\d{2}-(?:[a-f0-9]{4}(?:-\d{3})?|\d{3})$")
        .is_ok_and(|re| re.is_match(value))
}

/// Validate work item `depends_on` declarations per [[RFC-0006:C-DEPENDENCY-SEMANTICS]].
pub fn validate_work_dependencies(index: &ProjectIndex, config: &Config) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_work_ids: HashSet<&str> = index
        .work_items
        .iter()
        .map(|work| work.meta().id.as_str())
        .collect();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut path_by_id: HashMap<String, String> = HashMap::new();

    for work in &index.work_items {
        let work_id = work.meta().id.clone();
        graph.insert(work_id.clone(), work.meta().depends_on.clone());
        path_by_id.insert(
            work_id.clone(),
            config.display_path(&work.path).display().to_string(),
        );

        for dependency in &work.meta().depends_on {
            if !is_work_item_id(dependency) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0409WorkDependencyInvalid,
                    format!(
                        "Work item '{}' dependency '{}' must be a work item ID",
                        work.meta().id,
                        dependency
                    ),
                    config.display_path(&work.path).display().to_string(),
                ));
                continue;
            }

            if !known_work_ids.contains(dependency.as_str()) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0410WorkDependencyNotFound,
                    format!(
                        "Work item '{}' declares unknown work item dependency: {}",
                        work.meta().id,
                        dependency
                    ),
                    config.display_path(&work.path).display().to_string(),
                ));
            }
        }
    }

    let mut state = HashMap::new();
    let mut stack = Vec::new();
    let mut ids: Vec<_> = graph.keys().cloned().collect();
    ids.sort();
    for work_id in ids {
        if state.contains_key(&work_id) {
            continue;
        }
        if let Some(cycle) = detect_work_dependency_cycle(&work_id, &graph, &mut state, &mut stack)
        {
            let cycle_id = cycle.first().cloned().unwrap_or(work_id);
            let path = path_by_id.get(&cycle_id).cloned().unwrap_or_else(|| {
                config
                    .display_path(&config.work_dir())
                    .display()
                    .to_string()
            });
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0411WorkDependencyCycle,
                format!(
                    "cyclic work item dependency detected: {}",
                    cycle.join(" -> ")
                ),
                path,
            ));
            break;
        }
    }

    diagnostics
}

fn detect_work_dependency_cycle(
    work_id: &str,
    graph: &HashMap<String, Vec<String>>,
    state: &mut HashMap<String, WorkDependencyVisit>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    if state.get(work_id) == Some(&WorkDependencyVisit::Visiting) {
        let start = stack.iter().position(|id| id == work_id).unwrap_or(0);
        let mut cycle = stack[start..].to_vec();
        cycle.push(work_id.to_string());
        return Some(cycle);
    }
    if state.get(work_id) == Some(&WorkDependencyVisit::Visited) {
        return None;
    }

    state.insert(work_id.to_string(), WorkDependencyVisit::Visiting);
    stack.push(work_id.to_string());

    if let Some(dependencies) = graph.get(work_id) {
        for dependency in dependencies {
            if graph.contains_key(dependency)
                && let Some(cycle) = detect_work_dependency_cycle(dependency, graph, state, stack)
            {
                return Some(cycle);
            }
        }
    }

    stack.pop();
    state.insert(work_id.to_string(), WorkDependencyVisit::Visited);
    None
}
