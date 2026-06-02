use super::{output::print_loop, state::ensure_work_values};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::loop_planner::replan_loop_state_from_config;
use crate::loop_state::{LoopLifecycleState, LoopState, load_loop_state, write_loop_state_with_op};
use crate::write::WriteOp;
use std::collections::BTreeSet;

const WORK_FIELD: &str = "work";
const WI_FIELD_ALIAS: &str = "wi";

pub fn replan(config: &Config, loop_id: &str, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    mutate_scope(config, loop_id, ScopeMutation::Replan, &[], op)
}

pub fn add_work_item(
    config: &Config,
    loop_id: &str,
    field: &str,
    work_item: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    ensure_work_field(field)?;
    let work_ids = [work_item.to_string()];
    mutate_scope(config, loop_id, ScopeMutation::Add, &work_ids, op)
}

pub fn remove_work_item(
    config: &Config,
    loop_id: &str,
    field: &str,
    work_item: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    ensure_work_field(field)?;
    let work_ids = [work_item.to_string()];
    mutate_scope(config, loop_id, ScopeMutation::Remove, &work_ids, op)
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
    work: &[String],
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let state = load_loop_state(config, loop_id)?;
    ensure_loop_can_mutate_scope(&state)?;
    let updated_work = mutated_work_set(&state, mutation, work)?;
    let plan = replan_loop_state_from_config(config, &state, &updated_work)?;
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

fn mutated_work_set(
    state: &LoopState,
    mutation: ScopeMutation,
    work: &[String],
) -> DiagnosticResult<Vec<String>> {
    match mutation {
        ScopeMutation::Replan => {
            if !work.is_empty() {
                ensure_work_values(work)?;
            }
            Ok(state.loop_meta.work.clone())
        }
        ScopeMutation::Add => {
            ensure_work_values(work)?;
            let mut roots = work_root_set(state);
            roots.extend(work.iter().cloned());
            Ok(roots.into_iter().collect())
        }
        ScopeMutation::Remove => {
            ensure_work_values(work)?;
            let mut roots = work_root_set(state);
            for work_id in work {
                if !roots.remove(work_id) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E1209LoopWorkMismatch,
                        format!("Loop work field does not contain item to remove: {work_id}"),
                        state.loop_meta.id.clone(),
                    ));
                }
            }
            if roots.is_empty() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0801MissingRequiredArg,
                    "Loop work field must not be empty after remove",
                    state.loop_meta.id.clone(),
                ));
            }
            Ok(roots.into_iter().collect())
        }
    }
}

fn work_root_set(state: &LoopState) -> BTreeSet<String> {
    state.loop_meta.work.iter().cloned().collect()
}

fn ensure_work_field(field: &str) -> DiagnosticResult<()> {
    if matches!(field, WORK_FIELD | WI_FIELD_ALIAS) {
        return Ok(());
    }
    Err(Diagnostic::new(
        DiagnosticCode::E0803UnknownField,
        format!("Unknown loop field: {field}"),
        "loop",
    ))
}
