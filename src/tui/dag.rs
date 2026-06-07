use crate::diagnostic::DiagnosticResult;
use crate::loop_planner::topological_order_for_state;
use crate::loop_state::LoopState;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagLine {
    pub work_id: String,
    pub depth: usize,
    pub selected: bool,
    pub hidden: bool,
    pub text: String,
}

pub fn dag_lines(
    state: &LoopState,
    selected_work_id: Option<&str>,
    max_lines: usize,
) -> DiagnosticResult<Vec<DagLine>> {
    // Implements [[RFC-0007:C-LOOP-DAG]]: deterministic loop ordering.
    let order = topological_order_for_state(state)?;
    if max_lines == 0 {
        return Ok(Vec::new());
    }

    // Implements [[RFC-0007:C-LOOP-DAG]]: reserve space for hidden-count fallback.
    let item_budget = if order.len() > max_lines {
        max_lines.saturating_sub(1)
    } else {
        max_lines
    };
    // Implements [[RFC-0007:C-LOOP-DAG]]: selected-item neighborhood fallback.
    let visible = visible_work_ids(state, &order, selected_work_id, item_budget);
    // Implements [[RFC-0007:C-LOOP-DAG]]: hidden item counts remain visible.
    let hidden_count = order.len().saturating_sub(visible.len());
    // Implements [[RFC-0007:C-LOOP-DAG]]: stable visual depth for dependencies.
    let depths = node_depths(state, &order);

    let mut lines = Vec::new();
    for work_id in visible {
        let Some(item) = state.items.get(&work_id) else {
            continue;
        };
        let depth = *depths.get(&work_id).unwrap_or(&0);
        let selected = selected_work_id == Some(work_id.as_str());
        let deps = state
            .dependencies
            .get(&work_id)
            .filter(|deps| !deps.is_empty())
            .map(|deps| format!(" <- {}", deps.join(", ")))
            .unwrap_or_default();
        let branch = if depth == 0 { "●" } else { "└─" };
        let marker = if selected { ">" } else { " " };
        let indent = "  ".repeat(depth);
        lines.push(DagLine {
            work_id: work_id.clone(),
            depth,
            selected,
            hidden: false,
            text: format!(
                "{marker}{indent}{branch} {work_id} [{}]{}",
                item.status.as_str(),
                deps
            ),
        });
    }

    if hidden_count > 0 && lines.len() < max_lines {
        lines.push(DagLine {
            work_id: String::new(),
            depth: 0,
            selected: false,
            hidden: true,
            text: format!("... {hidden_count} hidden item(s); narrow DAG neighborhood shown"),
        });
    }

    Ok(lines)
}

fn visible_work_ids(
    state: &LoopState,
    order: &[String],
    selected_work_id: Option<&str>,
    max_lines: usize,
) -> Vec<String> {
    if order.len() <= max_lines {
        return order.to_vec();
    }

    let mut visible = BTreeSet::new();
    if let Some(selected) =
        selected_work_id.filter(|selected| order.iter().any(|id| id == selected))
    {
        visible.insert(selected.to_string());

        if max_lines > 1
            && let Some(deps) = state.dependencies.get(selected)
        {
            for dep in order.iter().filter(|work_id| deps.contains(*work_id)) {
                if visible.len() >= max_lines {
                    break;
                }
                visible.insert(dep.clone());
            }
        }

        if visible.len() < max_lines {
            for work_id in order {
                if visible.len() >= max_lines {
                    break;
                }
                if state
                    .dependencies
                    .get(work_id)
                    .is_some_and(|deps| deps.iter().any(|dep| dep == selected))
                {
                    visible.insert(work_id.clone());
                }
            }
        }
    }

    for work_id in order {
        if visible.len() >= max_lines {
            break;
        }
        visible.insert(work_id.clone());
    }

    order
        .iter()
        .filter(|work_id| visible.contains(*work_id))
        .take(max_lines)
        .cloned()
        .collect()
}

fn node_depths(state: &LoopState, order: &[String]) -> BTreeMap<String, usize> {
    let mut depths = BTreeMap::new();
    for work_id in order {
        let depth = state
            .dependencies
            .get(work_id)
            .map(|deps| {
                deps.iter()
                    .filter_map(|dep| depths.get(dep))
                    .map(|depth| depth + 1)
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        depths.insert(work_id.clone(), depth);
    }
    depths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loop_state::{LoopState, LoopWorkItemStatus};
    use std::collections::BTreeMap;

    #[test]
    fn dag_lines_are_deterministic_and_status_aware() -> Result<(), Box<dyn std::error::Error>> {
        let state = sample_loop_state()?;
        let lines = dag_lines(&state, Some("WI-2026-01-01-002"), 10)?;

        assert_eq!(lines[0].work_id, "WI-2026-01-01-001");
        assert!(lines.iter().any(|line| line.selected));
        assert!(
            lines
                .iter()
                .any(|line| line.text.contains("[active]")
                    && line.text.contains("WI-2026-01-01-002"))
        );
        Ok(())
    }

    #[test]
    fn dag_lines_show_neighborhood_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let state = sample_loop_state()?;
        let lines = dag_lines(&state, Some("WI-2026-01-01-002"), 2)?;

        assert!(lines.iter().any(|line| line.hidden));
        assert!(lines.len() <= 2);
        assert!(lines.iter().any(|line| line.selected));
        Ok(())
    }

    #[test]
    fn dag_lines_fallback_keeps_late_selected_item() -> Result<(), Box<dyn std::error::Error>> {
        let state = sample_loop_state()?;
        let lines = dag_lines(&state, Some("WI-2026-01-01-003"), 2)?;

        assert!(
            lines
                .iter()
                .any(|line| line.selected && line.work_id == "WI-2026-01-01-003")
        );
        assert!(lines.iter().any(|line| line.hidden));
        assert!(lines.len() <= 2);
        Ok(())
    }

    #[test]
    fn dag_lines_one_line_fallback_shows_hidden_count() -> Result<(), Box<dyn std::error::Error>> {
        let state = sample_loop_state()?;
        let lines = dag_lines(&state, Some("WI-2026-01-01-003"), 1)?;

        assert_eq!(lines.len(), 1);
        assert!(lines[0].hidden);
        assert!(lines[0].text.contains("hidden item"));
        Ok(())
    }

    fn sample_loop_state() -> DiagnosticResult<LoopState> {
        let mut dependencies = BTreeMap::new();
        dependencies.insert("WI-2026-01-01-001".to_string(), vec![]);
        dependencies.insert(
            "WI-2026-01-01-002".to_string(),
            vec!["WI-2026-01-01-001".to_string()],
        );
        dependencies.insert(
            "WI-2026-01-01-003".to_string(),
            vec!["WI-2026-01-01-002".to_string()],
        );
        let mut state = LoopState::new(
            "LOOP-2026-01-01-001",
            vec!["WI-2026-01-01-003".to_string()],
            vec![
                "WI-2026-01-01-001".to_string(),
                "WI-2026-01-01-002".to_string(),
                "WI-2026-01-01-003".to_string(),
            ],
            dependencies,
        )?;
        state.set_item_status("WI-2026-01-01-002", LoopWorkItemStatus::Active)?;
        Ok(state)
    }
}
