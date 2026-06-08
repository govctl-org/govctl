//! Loop command surface for local execution state.

mod execution;
mod output;
mod scope;
mod state;

pub use execution::run;
pub use scope::{add_work_item, remove_work_item, replan};

use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::{DiagnosticResult, Diagnostics};
use crate::loop_planner::build_loop_plan_from_config;
use crate::loop_state::{
    LoopLifecycleState, LoopState, load_loop_state, validate_loop_id, write_loop_state_with_op,
};
use crate::write::WriteOp;
use output::{LoopListEntry, print_loop, print_loop_list, print_loop_with_plan};
use state::{
    canonical_loop_ids, ensure_loop_not_terminal, ensure_work_values, find_reusable_loop,
    generated_loop_id, loop_plan_status, loop_plan_status_from_work_items,
};

pub fn start(
    config: &Config,
    loop_id: Option<&str>,
    work: &[String],
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    ensure_work_values(work)?;

    if let Some(existing) = find_reusable_loop(config, loop_id, work)? {
        print_loop("Reused", &existing)?;
        return Ok(vec![]);
    }

    let loop_id = match loop_id {
        Some(loop_id) => loop_id.to_string(),
        None => generated_loop_id(config)?,
    };
    validate_loop_id(&loop_id)?;

    let plan = build_loop_plan_from_config(config, &loop_id, work)?;
    write_loop_state_with_op(config, &plan.state, op)?;
    let verb = if op.is_preview() {
        "Would start"
    } else {
        "Started"
    };
    print_loop(verb, &plan.state)?;
    Ok(vec![])
}

pub fn show(config: &Config, loop_id: &str) -> DiagnosticResult<Diagnostics> {
    let state = load_loop_state(config, loop_id)?;
    let plan_status = loop_plan_status(config, &state);
    print_loop_with_plan("Loop", &state, plan_status.as_str())?;
    Ok(vec![])
}

pub fn list(
    config: &Config,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
) -> DiagnosticResult<Diagnostics> {
    let mut states = canonical_loop_ids(config)?
        .into_iter()
        .map(|loop_id| load_loop_state(config, &loop_id))
        .collect::<DiagnosticResult<Vec<_>>>()?;
    if let Some(filter) = filter {
        states.retain(|state| loop_list_filter_matches(state, filter));
    }
    if let Some(limit) = limit {
        states.truncate(limit);
    }
    let all_work_items = crate::parse::load_work_items(config).ok();
    let entries = states
        .iter()
        .map(|state| {
            let plan_status = all_work_items
                .as_deref()
                .map_or(state::LoopPlanStatus::Stale, |work_items| {
                    loop_plan_status_from_work_items(state, work_items)
                });
            LoopListEntry::from_state(state, plan_status.as_str())
        })
        .collect::<Vec<_>>();
    print_loop_list(&entries, output);
    Ok(vec![])
}

fn loop_list_filter_matches(state: &LoopState, filter: &str) -> bool {
    let normalized = filter.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "all" => true,
        "open" | "resumable" | "non-terminal" | "nonterminal" => matches!(
            state.loop_meta.state,
            LoopLifecycleState::Pending | LoopLifecycleState::Active | LoopLifecycleState::Paused
        ),
        "pending" | "active" | "paused" | "completed" | "failed" => {
            state.loop_meta.state.as_str() == normalized
        }
        _ => {
            state.loop_meta.id.contains(filter)
                || state
                    .loop_meta
                    .work
                    .iter()
                    .any(|work_id| work_id.contains(filter))
        }
    }
}

pub fn resume(config: &Config, loop_id: &str) -> DiagnosticResult<Diagnostics> {
    let state = load_loop_state(config, loop_id)?;
    ensure_loop_not_terminal(&state, "resume")?;
    print_loop("Resumed", &state)?;
    Ok(vec![])
}
