use super::item::execute_work_item_round;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_planner::{propagate_blocked_outcomes, topological_order_for_state};
use crate::loop_state::{
    LoopLifecycleState, LoopState, LoopWorkItemStatus, write_loop_state_with_op,
};
use crate::write::WriteOp;

pub(super) fn execute_run_round(
    config: &Config,
    state: &mut LoopState,
    max_rounds: u32,
    op: WriteOp,
    failures: &mut Vec<String>,
) -> DiagnosticResult<()> {
    for work_id in topological_order_for_state(state)? {
        propagate_blocked_outcomes(state)?;
        if is_terminal_item(state, &work_id) {
            continue;
        }
        match dependency_readiness(state, &work_id)? {
            DependencyReadiness::Ready => {}
            DependencyReadiness::Waiting => continue,
            DependencyReadiness::Blocked => {
                state.set_item_status(&work_id, LoopWorkItemStatus::Blocked)?;
                continue;
            }
        }

        if let Some(failure) = execute_work_item_round(config, state, &work_id, max_rounds, op)? {
            failures.push(failure);
            propagate_blocked_outcomes(state)?;
        }
        write_loop_state_with_op(config, state, op)?;
    }
    Ok(())
}

pub(super) fn finalize_run_state(state: &mut LoopState) -> DiagnosticResult<()> {
    propagate_blocked_outcomes(state)?;
    let has_failed = state.items.values().any(|item| {
        matches!(
            item.status,
            LoopWorkItemStatus::Failed | LoopWorkItemStatus::Blocked
        )
    });
    let all_terminal = state.items.values().all(|item| {
        matches!(
            item.status,
            LoopWorkItemStatus::Done
                | LoopWorkItemStatus::Failed
                | LoopWorkItemStatus::Blocked
                | LoopWorkItemStatus::Cancelled
        )
    });

    if has_failed {
        state.transition_to(LoopLifecycleState::Failed)
    } else if all_terminal {
        state.transition_to(LoopLifecycleState::Completed)
    } else {
        state.transition_to(LoopLifecycleState::Paused)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyReadiness {
    Ready,
    Waiting,
    Blocked,
}

fn dependency_readiness(state: &LoopState, work_id: &str) -> DiagnosticResult<DependencyReadiness> {
    let dependencies = state.dependencies.get(work_id).ok_or_else(|| {
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
        return Ok(DependencyReadiness::Blocked);
    }
    if dependencies
        .iter()
        .all(|dependency| state.items[dependency.as_str()].status == LoopWorkItemStatus::Done)
    {
        Ok(DependencyReadiness::Ready)
    } else {
        Ok(DependencyReadiness::Waiting)
    }
}

fn is_terminal_item(state: &LoopState, work_id: &str) -> bool {
    matches!(
        state.items[work_id].status,
        LoopWorkItemStatus::Done
            | LoopWorkItemStatus::Failed
            | LoopWorkItemStatus::Blocked
            | LoopWorkItemStatus::Cancelled
    )
}

pub(super) fn loop_failure_message(state: &LoopState, failures: &[String]) -> String {
    if failures.is_empty() {
        format!("Loop '{}' failed", state.loop_meta.id)
    } else {
        format!(
            "Loop '{}' failed:\n{}",
            state.loop_meta.id,
            failures
                .iter()
                .map(|failure| format!("  - {failure}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}
