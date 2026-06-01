use super::{output::print_loop, state::ensure_root_work_items};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::loop_planner::replan_loop_state_from_config;
use crate::loop_state::{LoopLifecycleState, LoopState, load_loop_state, write_loop_state_with_op};
use crate::write::WriteOp;
use std::collections::BTreeSet;

pub fn replan(config: &Config, loop_id: &str, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    mutate_scope(config, loop_id, ScopeMutation::Replan, &[], op)
}

pub fn add_roots(
    config: &Config,
    loop_id: &str,
    root_work_items: &[String],
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    mutate_scope(config, loop_id, ScopeMutation::Add, root_work_items, op)
}

pub fn remove_roots(
    config: &Config,
    loop_id: &str,
    root_work_items: &[String],
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    mutate_scope(config, loop_id, ScopeMutation::Remove, root_work_items, op)
}

#[derive(Debug, Clone, Copy)]
enum ScopeMutation {
    Replan,
    Add,
    Remove,
}

fn mutate_scope(
    config: &Config,
    loop_id: &str,
    mutation: ScopeMutation,
    root_work_items: &[String],
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let state = load_loop_state(config, loop_id)?;
    ensure_loop_can_mutate_scope(&state)?;
    let roots = mutated_root_set(&state, mutation, root_work_items)?;
    let plan = replan_loop_state_from_config(config, &state, &roots)?;
    write_loop_state_with_op(config, &plan.state, op)?;
    let verb = if op.is_preview() {
        match mutation {
            ScopeMutation::Replan => "Would replan",
            ScopeMutation::Add | ScopeMutation::Remove => "Would update",
        }
    } else {
        match mutation {
            ScopeMutation::Replan => "Replanned",
            ScopeMutation::Add | ScopeMutation::Remove => "Updated",
        }
    };
    print_loop(verb, &plan.state)?;
    Ok(vec![])
}

fn ensure_loop_can_mutate_scope(state: &LoopState) -> DiagnosticResult<()> {
    if matches!(
        state.loop_meta.state,
        LoopLifecycleState::Completed | LoopLifecycleState::Failed
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1210LoopExecutionFailed,
            format!(
                "Cannot mutate terminal loop '{}' in {} state",
                state.loop_meta.id,
                state.loop_meta.state.as_str()
            ),
            state.loop_meta.id.clone(),
        ));
    }
    Ok(())
}

fn mutated_root_set(
    state: &LoopState,
    mutation: ScopeMutation,
    root_work_items: &[String],
) -> DiagnosticResult<Vec<String>> {
    match mutation {
        ScopeMutation::Replan => {
            if !root_work_items.is_empty() {
                ensure_root_work_items(root_work_items)?;
            }
            Ok(state.loop_meta.root_work_items.clone())
        }
        ScopeMutation::Add => {
            ensure_root_work_items(root_work_items)?;
            let mut roots = state
                .loop_meta
                .root_work_items
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            roots.extend(root_work_items.iter().cloned());
            Ok(roots.into_iter().collect())
        }
        ScopeMutation::Remove => {
            ensure_root_work_items(root_work_items)?;
            let mut roots = state
                .loop_meta
                .root_work_items
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            for work_id in root_work_items {
                if !roots.remove(work_id) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E1209LoopRootMismatch,
                        format!(
                            "Loop root work item set does not contain root to remove: {work_id}"
                        ),
                        state.loop_meta.id.clone(),
                    ));
                }
            }
            if roots.is_empty() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0801MissingRequiredArg,
                    "Loop root work item set must not be empty after scope mutation",
                    state.loop_meta.id.clone(),
                ));
            }
            Ok(roots.into_iter().collect())
        }
    }
}
