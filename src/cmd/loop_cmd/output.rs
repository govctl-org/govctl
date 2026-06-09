use super::state::loop_item_state;
use crate::OutputFormat;
use crate::cmd::output::{print_json_array, table_with_bold_headers};
use crate::diagnostic::DiagnosticResult;
use crate::loop_planner::topological_order_for_state;
use crate::loop_state::LoopState;
use comfy_table::Cell;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct LoopListEntry {
    id: String,
    state: String,
    plan: String,
    work: Vec<String>,
    items: usize,
    rounds: u32,
    next_action: String,
}

impl LoopListEntry {
    pub(super) fn from_state(state: &LoopState, plan: &str) -> Self {
        Self {
            id: state.loop_meta.id.clone(),
            state: state.loop_meta.state.as_str().to_string(),
            plan: plan.to_string(),
            work: state.loop_meta.work.clone(),
            items: state.loop_meta.resolved.len(),
            rounds: state.items.values().map(|item| item.round_count).sum(),
            next_action: state.loop_meta.next_action.as_str().to_string(),
        }
    }

    fn work_display(&self) -> String {
        self.work.join(",")
    }
}

pub(super) fn print_loop_list(entries: &[LoopListEntry], output: OutputFormat) {
    match output {
        OutputFormat::Json => {
            print_json_array(entries);
        }
        OutputFormat::Plain => {
            for entry in entries {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    entry.id,
                    entry.state,
                    entry.plan,
                    entry.work_display(),
                    entry.items,
                    entry.rounds,
                    entry.next_action
                );
            }
        }
        OutputFormat::Table => {
            let mut table = table_with_bold_headers(&[
                "ID", "State", "Plan", "Work", "Items", "Rounds", "Action",
            ]);
            for entry in entries {
                table.add_row(vec![
                    Cell::new(&entry.id),
                    Cell::new(&entry.state),
                    Cell::new(&entry.plan),
                    Cell::new(entry.work_display()),
                    Cell::new(entry.items.to_string()),
                    Cell::new(entry.rounds.to_string()),
                    Cell::new(&entry.next_action),
                ]);
            }
            println!("{table}");
        }
    }
}

pub(super) fn print_loop(verb: &str, state: &LoopState) -> DiagnosticResult<()> {
    print_loop_inner(verb, state, None)
}

pub(super) fn print_loop_with_plan(
    verb: &str,
    state: &LoopState,
    plan: &str,
) -> DiagnosticResult<()> {
    print_loop_inner(verb, state, Some(plan))
}

fn print_loop_inner(verb: &str, state: &LoopState, plan: Option<&str>) -> DiagnosticResult<()> {
    if verb == "Loop" {
        println!("Loop {}", state.loop_meta.id);
    } else {
        println!("{} loop {}", verb, state.loop_meta.id);
    }
    println!("State: {}", state.loop_meta.state.as_str());
    if let Some(plan) = plan {
        println!("Plan status: {plan}");
    }
    println!("Current round: {}", state.loop_meta.current_round);
    println!("Next action: {}", state.loop_meta.next_action.as_str());
    println!("Work: {}", state.loop_meta.work.join(", "));
    println!("Resolved: {} work item(s)", state.loop_meta.resolved.len());
    println!("Plan:");
    for (index, work_id) in topological_order_for_state(state)?.iter().enumerate() {
        let item = loop_item_state(state, work_id)?;
        let deps = state
            .dependencies
            .get(work_id)
            .filter(|deps| !deps.is_empty())
            .map(|deps| deps.join(","))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {}. {} status={} rounds={} last_round={} depends_on={}",
            index + 1,
            work_id,
            item.status.as_str(),
            item.round_count,
            item.last_round,
            deps
        );
    }
    Ok(())
}
