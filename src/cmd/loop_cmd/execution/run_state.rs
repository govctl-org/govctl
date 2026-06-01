use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_state::{LoopLifecycleState, LoopState, load_loop_state};
use std::collections::BTreeSet;

pub(super) fn state_for_run(
    config: &Config,
    loop_id: &str,
    target_work_items: &[String],
) -> DiagnosticResult<LoopState> {
    let state = load_loop_state(config, loop_id)?;
    validate_target_work_items(&state, target_work_items)?;
    Ok(state)
}

fn validate_target_work_items(
    state: &LoopState,
    target_work_items: &[String],
) -> DiagnosticResult<()> {
    let loop_work_items = state
        .loop_meta
        .resolved
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for work_id in target_work_items {
        if !crate::validate::is_work_item_id(work_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0409WorkDependencyInvalid,
                format!("Loop run target '{work_id}' must be a work item ID"),
                state.loop_meta.id.clone(),
            ));
        }
        if !seen.insert(work_id.as_str()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("duplicate loop run target work item: {work_id}"),
                state.loop_meta.id.clone(),
            ));
        }
        if !loop_work_items.contains(work_id.as_str()) {
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
