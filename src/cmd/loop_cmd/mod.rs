//! Loop command surface for local execution state.

mod execution;
mod output;
mod scope;
mod state;

pub use execution::run;
pub use scope::{add_roots, remove_roots, replan};

use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_planner::build_loop_plan_from_config;
use crate::loop_state::{load_loop_state, validate_loop_id, write_loop_state_with_op};
use crate::write::WriteOp;
use output::{LoopListEntry, print_loop, print_loop_list};
use state::{
    canonical_loop_ids, ensure_root_work_items, ensure_same_root_set,
    find_matching_non_terminal_loop, find_reusable_loop, generated_loop_id,
};

pub fn start(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    ensure_root_work_items(root_work_items)?;

    if let Some(existing) = find_reusable_loop(config, loop_id, root_work_items)? {
        print_loop("Reused", &existing)?;
        return Ok(vec![]);
    }

    let loop_id = match loop_id {
        Some(loop_id) => loop_id.to_string(),
        None => generated_loop_id(config)?,
    };
    validate_loop_id(&loop_id)?;

    let plan = build_loop_plan_from_config(config, &loop_id, root_work_items)?;
    write_loop_state_with_op(config, &plan.state, op)?;
    let verb = if op.is_preview() {
        "Would start"
    } else {
        "Started"
    };
    print_loop(verb, &plan.state)?;
    Ok(vec![])
}

pub fn show(config: &Config, loop_id: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let state = load_loop_state(config, loop_id)?;
    print_loop("Loop", &state)?;
    Ok(vec![])
}

pub fn list(config: &Config, output: OutputFormat) -> anyhow::Result<Vec<Diagnostic>> {
    let states = canonical_loop_ids(config)?
        .into_iter()
        .map(|loop_id| load_loop_state(config, &loop_id))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let entries = states
        .iter()
        .map(LoopListEntry::from_state)
        .collect::<Vec<_>>();
    print_loop_list(&entries, output);
    Ok(vec![])
}

pub fn resume(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
) -> anyhow::Result<Vec<Diagnostic>> {
    let state = if let Some(loop_id) = loop_id {
        let state = load_loop_state(config, loop_id)?;
        if !root_work_items.is_empty() {
            ensure_root_work_items(root_work_items)?;
            ensure_same_root_set(&state, root_work_items)?;
        }
        state
    } else {
        ensure_root_work_items(root_work_items)?;
        find_matching_non_terminal_loop(config, root_work_items)?.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1207LoopResumeNotFound,
                "No matching non-terminal loop state found; start a new loop or pass --id",
                root_work_items.join(", "),
            )
        })?
    };

    print_loop("Resumed", &state)?;
    Ok(vec![])
}
