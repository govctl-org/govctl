use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_state::LoopLifecycleState;

pub(in crate::loop_state) fn validate_loop_transition(
    loop_id: &str,
    from: LoopLifecycleState,
    to: LoopLifecycleState,
) -> DiagnosticResult<()> {
    if is_valid_loop_transition(from, to) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1203LoopInvalidTransition,
            format!("Invalid loop transition: {from:?} -> {to:?}"),
            loop_id,
        ))
    }
}

fn is_valid_loop_transition(from: LoopLifecycleState, to: LoopLifecycleState) -> bool {
    matches!(
        (from, to),
        (LoopLifecycleState::Pending, LoopLifecycleState::Active)
            | (LoopLifecycleState::Active, LoopLifecycleState::Paused)
            | (LoopLifecycleState::Paused, LoopLifecycleState::Active)
            | (LoopLifecycleState::Active, LoopLifecycleState::Completed)
            | (LoopLifecycleState::Active, LoopLifecycleState::Failed)
            | (LoopLifecycleState::Paused, LoopLifecycleState::Failed)
    )
}
