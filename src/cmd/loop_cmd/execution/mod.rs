mod item;
mod round;
mod run_state;

use super::output::print_loop;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_state::{LoopLifecycleState, write_loop_state_with_op};
use crate::write::WriteOp;
use round::{execute_run_round, finalize_run_state, loop_failure_message};
use run_state::{ensure_loop_can_run, enter_active_state, state_for_run};

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
