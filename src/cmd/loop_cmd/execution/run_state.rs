use crate::cmd::loop_cmd::state::ensure_unique_work_item_ids;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_state::{LoopLifecycleState, LoopState, load_loop_state};
use std::collections::BTreeSet;

pub(super) fn state_for_run(
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

pub(super) fn ensure_loop_can_run(state: &LoopState) -> DiagnosticResult<()> {
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
        ));
    }
    Ok(())
}

pub(super) fn enter_active_state(state: &mut LoopState) -> DiagnosticResult<()> {
    match state.loop_meta.state {
        LoopLifecycleState::Pending | LoopLifecycleState::Paused => {
            state.transition_to(LoopLifecycleState::Active)
        }
        LoopLifecycleState::Active => Ok(()),
        LoopLifecycleState::Completed | LoopLifecycleState::Failed => ensure_loop_can_run(state),
    }
}
