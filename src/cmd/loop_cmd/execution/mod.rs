mod item;
mod round;
mod run_state;

use super::output::print_loop;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::loop_state::{LoopLifecycleState, write_loop_state_with_op};
use crate::write::WriteOp;
use round::{execute_run_round, finalize_run_state, loop_failure_message};
use run_state::{ensure_loop_can_run, enter_active_state, state_for_run};

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
    ensure_loop_can_run(&state)?;

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
