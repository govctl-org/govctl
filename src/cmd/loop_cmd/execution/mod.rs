use super::output::print_loop;
use super::state::{
    ensure_loop_not_terminal, ensure_unique_work_item_ids, loop_dependencies, loop_item_state,
};
use crate::cmd::work_lookup::load_work_item_by_id;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::loop_planner::{propagate_blocked_outcomes, topological_order_for_state};
use crate::loop_state::{
    LoopLifecycleState, LoopNextAction, LoopRoundRecord, LoopRoundStatus, LoopState,
    LoopWorkItemStatus, load_loop_round_record, load_loop_state, loop_round_path,
    write_loop_round_record, write_loop_state_with_op,
};
use crate::model::WorkItemStatus;
use crate::write::WriteOp;
use std::collections::BTreeSet;

pub fn run(
    config: &Config,
    loop_id: &str,
    target_work_ids: &[String],
    max_rounds: u32,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if max_rounds == 0 {
        return Err(Diagnostic::new(
            DiagnosticCode::E1211LoopInvalidMaxRounds,
            "Loop max rounds must be at least 1",
            "loop",
        ));
    }

    let mut state = state_for_run(config, loop_id, target_work_ids)?;
    ensure_loop_not_terminal(&state, "run")?;

    println!("Running loop {}", state.loop_meta.id);
    println!("Max rounds: {max_rounds}");
    if !target_work_ids.is_empty() {
        println!("Targets: {}", target_work_ids.join(", "));
    }

    enter_active_state(&mut state)?;
    match current_round_record(config, &state)? {
        Some(record) if record.round_meta.status != LoopRoundStatus::Closed => {
            close_round(config, &mut state, record, target_work_ids, op)
        }
        _ => open_round(config, &mut state, target_work_ids, max_rounds, op),
    }
}

fn state_for_run(
    config: &Config,
    loop_id: &str,
    target_work_ids: &[String],
) -> DiagnosticResult<LoopState> {
    let state = load_loop_state(config, loop_id)?;
    validate_target_work_ids(&state, target_work_ids)?;
    Ok(state)
}

fn validate_target_work_ids(state: &LoopState, target_work_ids: &[String]) -> DiagnosticResult<()> {
    let loop_work_ids = state
        .loop_meta
        .resolved
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    ensure_unique_work_item_ids(
        target_work_ids,
        "Loop run target",
        "loop run target work item",
        &state.loop_meta.id,
    )?;
    for work_id in target_work_ids {
        if !loop_work_ids.contains(work_id.as_str()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!(
                    "Loop run target '{work_id}' is not part of loop '{}'",
                    state.loop_meta.id
                ),
                state.loop_meta.id.clone(),
            ));
        }
    }
    Ok(())
}

fn enter_active_state(state: &mut LoopState) -> DiagnosticResult<()> {
    match state.loop_meta.state {
        LoopLifecycleState::Pending | LoopLifecycleState::Paused => {
            state.transition_to(LoopLifecycleState::Active)
        }
        LoopLifecycleState::Active => Ok(()),
        LoopLifecycleState::Completed | LoopLifecycleState::Failed => {
            ensure_loop_not_terminal(state, "run")
        }
    }
}

fn current_round_record(
    config: &Config,
    state: &LoopState,
) -> DiagnosticResult<Option<LoopRoundRecord>> {
    if state.loop_meta.current_round == 0 {
        return Ok(None);
    }
    match load_loop_round_record(config, &state.loop_meta.id, state.loop_meta.current_round) {
        Ok(record) => Ok(Some(record)),
        Err(err) if err.code == DiagnosticCode::E1202LoopStateNotFound => {
            let path = loop_round_path(config, &state.loop_meta.id, state.loop_meta.current_round)?;
            Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!(
                    "Loop state points to missing round artifact: {}",
                    config.display_path(&path).display()
                ),
                state.loop_meta.id.clone(),
            ))
        }
        Err(err) => Err(err),
    }
}

fn open_round(
    config: &Config,
    state: &mut LoopState,
    target_work_ids: &[String],
    max_rounds: u32,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    reflect_terminal_work_statuses(config, state)?;
    propagate_blocked_outcomes(state)?;

    let ready_work = ready_work_for_round(state, target_work_ids, max_rounds)?;
    if ready_work.failures.is_empty() && !ready_work.selected.is_empty() {
        let round_number = next_round_number(state)?;
        state.loop_meta.current_round = round_number;
        state.loop_meta.next_action = LoopNextAction::WriteSummary;
        for work_id in &ready_work.selected {
            state.set_item_status(work_id, LoopWorkItemStatus::Active)?;
            state.record_item_round(work_id, round_number)?;
        }

        let record = LoopRoundRecord::open(
            state.loop_meta.id.clone(),
            round_number,
            max_rounds,
            ready_work.selected,
        );
        write_loop_round_record(config, &record, op)?;
        write_loop_state_with_op(config, state, op)?;
        print_opened_round(config, &record)?;
        print_loop("Loop", state)?;
        return Ok(vec![]);
    }

    finalize_run_state(state)?;
    state.loop_meta.next_action = next_action_for_state(state);
    write_loop_state_with_op(config, state, op)?;
    print_final_state(state)?;
    if state.loop_meta.state == LoopLifecycleState::Failed {
        return Err(Diagnostic::new(
            DiagnosticCode::E1210LoopExecutionFailed,
            loop_failure_message(state, &ready_work.failures),
            state.loop_meta.id.clone(),
        ));
    }
    Ok(vec![])
}

fn close_round(
    config: &Config,
    state: &mut LoopState,
    mut record: LoopRoundRecord,
    target_work_ids: &[String],
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    validate_open_round_target_selector(state, &record, target_work_ids)?;
    if !record.has_required_summary_evidence() {
        let path = loop_round_path(
            config,
            &record.round_meta.loop_id,
            record.round_meta.round_number,
        )?;
        return Err(Diagnostic::new(
            DiagnosticCode::E1210LoopExecutionFailed,
            format!(
                "Loop round summary is incomplete; update {} before rerunning loop run",
                config.display_path(&path).display()
            ),
            record.round_meta.loop_id.clone(),
        ));
    }

    reflect_terminal_work_statuses(config, state)?;
    if record.summary.has_blockers() {
        state.transition_to(LoopLifecycleState::Paused)?;
        state.loop_meta.next_action = LoopNextAction::ResolveBlocker;
    } else {
        finalize_run_state(state)?;
        state.loop_meta.next_action = next_action_for_state(state);
    }
    record.round_meta.status = LoopRoundStatus::Closed;
    write_loop_round_record(config, &record, op)?;
    write_loop_state_with_op(config, state, op)?;
    print_final_state(state)?;
    if state.loop_meta.state == LoopLifecycleState::Failed {
        return Err(Diagnostic::new(
            DiagnosticCode::E1210LoopExecutionFailed,
            loop_failure_message(state, &[]),
            state.loop_meta.id.clone(),
        ));
    }
    Ok(vec![])
}

fn validate_open_round_target_selector(
    state: &LoopState,
    record: &LoopRoundRecord,
    target_work_ids: &[String],
) -> DiagnosticResult<()> {
    if target_work_ids.is_empty() {
        return Ok(());
    }
    let selected_work_ids = selected_execution_set(state, target_work_ids)?;
    for work_id in &record.round_meta.work {
        if !selected_work_ids.contains(work_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!(
                    "Loop run target selector does not include open round work item: {work_id}"
                ),
                state.loop_meta.id.clone(),
            ));
        }
    }
    Ok(())
}

struct ReadyWork {
    selected: Vec<String>,
    failures: Vec<String>,
}

fn ready_work_for_round(
    state: &mut LoopState,
    target_work_ids: &[String],
    max_rounds: u32,
) -> DiagnosticResult<ReadyWork> {
    let selected_work_ids = selected_execution_set(state, target_work_ids)?;
    let mut selected = Vec::new();
    let mut failures = Vec::new();

    for work_id in topological_order_for_state(state)? {
        propagate_blocked_outcomes(state)?;
        if !selected_work_ids.is_empty() && !selected_work_ids.contains(work_id.as_str()) {
            continue;
        }
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

        let current_rounds = loop_item_state(state, &work_id)?.round_count;
        if current_rounds >= max_rounds {
            state.set_item_status(&work_id, LoopWorkItemStatus::Failed)?;
            failures.push(format!("{work_id}: maximum rounds reached ({max_rounds})"));
            continue;
        }
        selected.push(work_id);
    }

    propagate_blocked_outcomes(state)?;
    Ok(ReadyWork { selected, failures })
}

fn selected_execution_set(
    state: &LoopState,
    target_work_ids: &[String],
) -> DiagnosticResult<BTreeSet<String>> {
    let mut selected = BTreeSet::new();
    for work_id in target_work_ids {
        collect_target_with_dependencies(state, work_id, &mut selected)?;
    }
    Ok(selected)
}

fn collect_target_with_dependencies(
    state: &LoopState,
    work_id: &str,
    selected: &mut BTreeSet<String>,
) -> DiagnosticResult<()> {
    if !selected.insert(work_id.to_string()) {
        return Ok(());
    }
    let dependencies = loop_dependencies(state, work_id, "selected work item")?;
    for dependency in dependencies {
        collect_target_with_dependencies(state, dependency, selected)?;
    }
    Ok(())
}

fn reflect_terminal_work_statuses(config: &Config, state: &mut LoopState) -> DiagnosticResult<()> {
    for work_id in state.loop_meta.resolved.clone() {
        match load_work_item_by_id(config, &work_id)?.spec.govctl.status {
            WorkItemStatus::Done => state.set_item_status(&work_id, LoopWorkItemStatus::Done)?,
            WorkItemStatus::Cancelled => {
                state.set_item_status(&work_id, LoopWorkItemStatus::Cancelled)?
            }
            WorkItemStatus::Queue | WorkItemStatus::Active => {}
        }
    }
    Ok(())
}

fn finalize_run_state(state: &mut LoopState) -> DiagnosticResult<()> {
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
    let dependencies = loop_dependencies(state, work_id, "work item")?;
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

fn next_round_number(state: &LoopState) -> DiagnosticResult<u32> {
    state.loop_meta.current_round.checked_add(1).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            "Loop current_round overflow",
            state.loop_meta.id.clone(),
        )
    })
}

fn next_action_for_state(state: &LoopState) -> LoopNextAction {
    match state.loop_meta.state {
        LoopLifecycleState::Completed => LoopNextAction::Complete,
        LoopLifecycleState::Failed => LoopNextAction::ResolveBlocker,
        LoopLifecycleState::Pending | LoopLifecycleState::Active | LoopLifecycleState::Paused => {
            LoopNextAction::Continue
        }
    }
}

fn print_opened_round(config: &Config, record: &LoopRoundRecord) -> DiagnosticResult<()> {
    let path = loop_round_path(
        config,
        &record.round_meta.loop_id,
        record.round_meta.round_number,
    )?;
    println!(
        "Opened round {} for loop {}",
        record.round_meta.round_number, record.round_meta.loop_id
    );
    println!("Round: {}", config.display_path(&path).display());
    println!(
        "Next action: fill summary evidence, then rerun `govctl loop run {}`",
        record.round_meta.loop_id
    );
    Ok(())
}

fn print_final_state(state: &LoopState) -> DiagnosticResult<()> {
    match state.loop_meta.state {
        LoopLifecycleState::Completed => print_loop("Completed", state),
        LoopLifecycleState::Paused => print_loop("Paused", state),
        LoopLifecycleState::Failed => print_loop("Failed", state),
        LoopLifecycleState::Pending | LoopLifecycleState::Active => print_loop("Loop", state),
    }
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
