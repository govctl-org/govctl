use super::super::state::{
    diagnostic_code, ensure_root_work_items, ensure_same_root_set, find_matching_non_terminal_loop,
    generated_loop_id,
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_planner::build_loop_plan_from_config;
use crate::loop_state::{LoopLifecycleState, LoopState, load_loop_state};

pub(super) fn state_for_run(
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

pub(super) fn ensure_loop_can_run(state: &LoopState) -> anyhow::Result<()> {
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

pub(super) fn enter_active_state(state: &mut LoopState) -> anyhow::Result<()> {
    match state.loop_meta.state {
        LoopLifecycleState::Pending | LoopLifecycleState::Paused => {
            state.transition_to(LoopLifecycleState::Active)
        }
        LoopLifecycleState::Active => Ok(()),
        LoopLifecycleState::Completed | LoopLifecycleState::Failed => ensure_loop_can_run(state),
    }
}
