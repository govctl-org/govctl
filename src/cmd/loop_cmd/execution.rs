use super::{
    output::print_loop,
    state::{
        diagnostic_code, ensure_root_work_items, ensure_same_root_set,
        find_matching_non_terminal_loop, generated_loop_id,
    },
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_planner::{
    build_loop_plan_from_config, propagate_blocked_outcomes, topological_order_for_state,
};
use crate::loop_state::{
    LoopLifecycleState, LoopRoundRecord, LoopState, LoopWorkItemStatus, load_loop_state,
    write_loop_round_record, write_loop_state_with_op,
};
use crate::model::{ChecklistStatus, WorkItemEntry, WorkItemStatus};
use crate::write::WriteOp;
use std::path::Path;

pub fn run(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
    max_rounds: u32,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    if max_rounds == 0 {
        return Err(Diagnostic::new(
            DiagnosticCode::E1211LoopInvalidMaxRounds,
            "Loop max rounds must be at least 1",
            "loop",
        )
        .into());
    }

    let mut state = state_for_run(config, loop_id, root_work_items)?;
    ensure_loop_can_run(&state)?;

    println!("Running loop {}", state.loop_meta.id);
    println!("Max rounds: {max_rounds}");

    enter_active_state(&mut state)?;
    write_loop_state_with_op(config, &state, op)?;

    let mut failures = Vec::new();
    execute_run_round(config, &mut state, max_rounds, op, &mut failures)?;
    finalize_run_state(&mut state)?;
    write_loop_state_with_op(config, &state, op)?;

    match state.loop_meta.state {
        LoopLifecycleState::Completed => {
            print_loop("Completed", &state)?;
            Ok(vec![])
        }
        LoopLifecycleState::Paused => {
            print_loop("Paused", &state)?;
            Ok(vec![])
        }
        LoopLifecycleState::Failed => {
            print_loop("Failed", &state)?;
            Err(Diagnostic::new(
                DiagnosticCode::E1210LoopExecutionFailed,
                loop_failure_message(&state, &failures),
                state.loop_meta.id.clone(),
            )
            .into())
        }
        LoopLifecycleState::Pending | LoopLifecycleState::Active => Ok(vec![]),
    }
}

fn state_for_run(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
) -> anyhow::Result<LoopState> {
    if let Some(loop_id) = loop_id {
        match load_loop_state(config, loop_id) {
            Ok(state) => {
                if !root_work_items.is_empty() {
                    ensure_root_work_items(root_work_items)?;
                    ensure_same_root_set(&state, root_work_items)?;
                }
                return Ok(state);
            }
            Err(err) if diagnostic_code(&err) == Some(DiagnosticCode::E1202LoopStateNotFound) => {
                if root_work_items.is_empty() {
                    return Err(err);
                }
            }
            Err(err) => return Err(err),
        }

        ensure_root_work_items(root_work_items)?;
        return Ok(build_loop_plan_from_config(config, loop_id, root_work_items)?.state);
    }

    ensure_root_work_items(root_work_items)?;
    if let Some(state) = find_matching_non_terminal_loop(config, root_work_items)? {
        Ok(state)
    } else {
        let loop_id = generated_loop_id(config)?;
        Ok(build_loop_plan_from_config(config, &loop_id, root_work_items)?.state)
    }
}

fn ensure_loop_can_run(state: &LoopState) -> anyhow::Result<()> {
    if matches!(
        state.loop_meta.state,
        LoopLifecycleState::Completed | LoopLifecycleState::Failed
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1210LoopExecutionFailed,
            format!(
                "Cannot run terminal loop '{}' in {} state",
                state.loop_meta.id,
                state.loop_meta.state.as_str()
            ),
            state.loop_meta.id.clone(),
        )
        .into());
    }
    Ok(())
}

fn enter_active_state(state: &mut LoopState) -> anyhow::Result<()> {
    match state.loop_meta.state {
        LoopLifecycleState::Pending | LoopLifecycleState::Paused => {
            state.transition_to(LoopLifecycleState::Active)
        }
        LoopLifecycleState::Active => Ok(()),
        LoopLifecycleState::Completed | LoopLifecycleState::Failed => ensure_loop_can_run(state),
    }
}

fn execute_run_round(
    config: &Config,
    state: &mut LoopState,
    max_rounds: u32,
    op: WriteOp,
    failures: &mut Vec<String>,
) -> anyhow::Result<()> {
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

fn execute_work_item_round(
    config: &Config,
    state: &mut LoopState,
    work_id: &str,
    max_rounds: u32,
    op: WriteOp,
) -> anyhow::Result<Option<String>> {
    let item_status_before = loop_item_status_string(state, work_id)?;
    let mut entry = load_work_item_by_id(config, work_id)?;
    let work_status_before = work_item_status_string(entry.spec.govctl.status);
    match entry.spec.govctl.status {
        WorkItemStatus::Done => {
            state.set_item_status(work_id, LoopWorkItemStatus::Done)?;
            return Ok(None);
        }
        WorkItemStatus::Cancelled => {
            state.set_item_status(work_id, LoopWorkItemStatus::Cancelled)?;
            return Ok(None);
        }
        WorkItemStatus::Queue => {
            if let Err(err) =
                crate::cmd::move_::move_item(config, Path::new(work_id), WorkItemStatus::Active, op)
            {
                state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
                return Ok(Some(format!(
                    "{work_id}: failed to transition to active: {}",
                    error_summary(&err)
                )));
            }
            if !op.is_preview() {
                entry = load_work_item_by_id(config, work_id)?;
            } else {
                entry.spec.govctl.status = WorkItemStatus::Active;
            }
        }
        WorkItemStatus::Active => {}
    }

    state.set_item_status(work_id, LoopWorkItemStatus::Active)?;
    let current_rounds = state
        .items
        .get(work_id)
        .map(|item| item.round_count)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("missing item state for work item: {work_id}"),
                state.loop_meta.id.clone(),
            )
        })?;
    if current_rounds >= max_rounds {
        state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
        return Ok(Some(format!(
            "{work_id}: maximum rounds reached ({max_rounds})"
        )));
    }
    let round = state.increment_round_count(work_id)?;

    let (action, reason, failure) = if acceptance_criteria_satisfied(&entry) {
        match crate::cmd::move_::move_item(config, Path::new(work_id), WorkItemStatus::Done, op) {
            Ok(_) => {
                state.set_item_status(work_id, LoopWorkItemStatus::Done)?;
                entry.spec.govctl.status = WorkItemStatus::Done;
                (
                    "evaluated acceptance criteria and completed work item".to_string(),
                    None,
                    None,
                )
            }
            Err(err) => {
                let summary = error_summary(&err);
                if round < max_rounds && is_retryable_guard_assertion_failure(&summary) {
                    (
                        "evaluated acceptance criteria and verification guards".to_string(),
                        Some(format!(
                            "verification guard assertion failed; max rounds not reached: {summary}"
                        )),
                        None,
                    )
                } else {
                    state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
                    let reason = format!("failed to complete after round {round}: {summary}");
                    (
                        "evaluated acceptance criteria and verification guards".to_string(),
                        Some(reason.clone()),
                        Some(format!("{work_id}: {reason}")),
                    )
                }
            }
        }
    } else if round >= max_rounds {
        state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
        let reason =
            format!("maximum rounds reached ({max_rounds}) with pending acceptance criteria");
        (
            "evaluated acceptance criteria".to_string(),
            Some(reason.clone()),
            Some(format!("{work_id}: {reason}")),
        )
    } else {
        (
            "evaluated acceptance criteria".to_string(),
            Some("pending acceptance criteria remain; max rounds not reached".to_string()),
            None,
        )
    };

    let item_status_after = loop_item_status_string(state, work_id)?;
    let work_status_after = if op.is_preview() {
        work_item_status_string(entry.spec.govctl.status)
    } else {
        work_item_status_string(load_work_item_by_id(config, work_id)?.spec.govctl.status)
    };
    write_loop_round_record(
        config,
        &LoopRoundRecord {
            loop_id: state.loop_meta.id.clone(),
            work_item_id: work_id.to_string(),
            round_number: round,
            max_rounds,
            item_status_before,
            item_status_after: item_status_after.clone(),
            work_status_before,
            work_status_after,
            action,
            outcome: item_status_after,
            reason,
        },
        op,
    )?;
    Ok(failure)
}

fn finalize_run_state(state: &mut LoopState) -> anyhow::Result<()> {
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

fn dependency_readiness(state: &LoopState, work_id: &str) -> anyhow::Result<DependencyReadiness> {
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

fn acceptance_criteria_satisfied(entry: &WorkItemEntry) -> bool {
    !entry.spec.content.acceptance_criteria.is_empty()
        && entry
            .spec
            .content
            .acceptance_criteria
            .iter()
            .all(|criterion| criterion.status != ChecklistStatus::Pending)
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

fn loop_item_status_string(state: &LoopState, work_id: &str) -> anyhow::Result<String> {
    state
        .items
        .get(work_id)
        .map(|item| item.status.as_str().to_string())
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("missing item state for work item: {work_id}"),
                state.loop_meta.id.clone(),
            )
            .into()
        })
}

fn work_item_status_string(status: WorkItemStatus) -> String {
    match status {
        WorkItemStatus::Queue => "queue",
        WorkItemStatus::Active => "active",
        WorkItemStatus::Done => "done",
        WorkItemStatus::Cancelled => "cancelled",
    }
    .to_string()
}

fn load_work_item_by_id(config: &Config, work_id: &str) -> anyhow::Result<WorkItemEntry> {
    crate::parse::load_work_items(config)?
        .into_iter()
        .find(|entry| entry.spec.govctl.id == work_id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {work_id}"),
                work_id,
            )
            .into()
        })
}

fn loop_failure_message(state: &LoopState, failures: &[String]) -> String {
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

fn error_summary(err: &anyhow::Error) -> String {
    if let Some(diagnostic) = err.downcast_ref::<Diagnostic>() {
        diagnostic.message.clone()
    } else {
        err.to_string()
    }
}

fn is_retryable_guard_assertion_failure(summary: &str) -> bool {
    summary.contains("verification guard requirements failed")
        && summary.contains("Verification guard '")
        && summary.contains(" failed")
        && !summary.contains("timed out")
        && !summary.contains("Failed to start")
        && !summary.contains("Unknown verification guard")
}
