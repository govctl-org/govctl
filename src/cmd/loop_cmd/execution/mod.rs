mod item;
mod round;

use super::output::print_loop;
use super::state::{ensure_loop_not_terminal, ensure_unique_work_item_ids};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::loop_state::{LoopLifecycleState, LoopState, load_loop_state, write_loop_state_with_op};
use crate::write::WriteOp;
use round::{execute_run_round, finalize_run_state, loop_failure_message};
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
    write_loop_state_with_op(config, &state, op)?;

    let mut failures = Vec::new();
    execute_run_round(
        config,
        &mut state,
        target_work_ids,
        max_rounds,
        op,
        &mut failures,
    )?;
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
            ))
        }
        LoopLifecycleState::Pending | LoopLifecycleState::Active => Ok(vec![]),
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
