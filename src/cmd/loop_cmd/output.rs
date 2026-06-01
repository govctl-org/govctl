use crate::OutputFormat;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_planner::topological_order_for_state;
use crate::loop_state::LoopState;
use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets::UTF8_FULL};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct LoopListEntry {
    id: String,
    state: String,
    work: Vec<String>,
    items: usize,
    rounds: u32,
}

impl LoopListEntry {
    pub(super) fn from_state(state: &LoopState) -> Self {
        Self {
            id: state.loop_meta.id.clone(),
            state: state.loop_meta.state.as_str().to_string(),
            work: state.loop_meta.work.clone(),
            items: state.loop_meta.resolved.len(),
            rounds: state.items.values().map(|item| item.round_count).sum(),
        }
    }

    fn work_display(&self) -> String {
        self.work.join(",")
    }
}

pub(super) fn print_loop_list(entries: &[LoopListEntry], output: OutputFormat) {
    match output {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string())
            );
        }
        OutputFormat::Plain => {
            for entry in entries {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    entry.id,
                    entry.state,
                    entry.work_display(),
                    entry.items,
                    entry.rounds
                );
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("ID").add_attribute(Attribute::Bold),
                    Cell::new("State").add_attribute(Attribute::Bold),
                    Cell::new("Work").add_attribute(Attribute::Bold),
                    Cell::new("Items").add_attribute(Attribute::Bold),
                    Cell::new("Rounds").add_attribute(Attribute::Bold),
                ]);
            for entry in entries {
                table.add_row(vec![
                    Cell::new(&entry.id),
                    Cell::new(&entry.state),
                    Cell::new(entry.work_display()),
                    Cell::new(entry.items.to_string()),
                    Cell::new(entry.rounds.to_string()),
                ]);
            }
            println!("{table}");
        }
    }
}

pub(super) fn print_loop(verb: &str, state: &LoopState) -> DiagnosticResult<()> {
    if verb == "Loop" {
        println!("Loop {}", state.loop_meta.id);
    } else {
        println!("{} loop {}", verb, state.loop_meta.id);
    }
    println!("State: {}", state.loop_meta.state.as_str());
    println!("Work: {}", state.loop_meta.work.join(", "));
    println!("Resolved: {} work item(s)", state.loop_meta.resolved.len());
    println!("Plan:");
    for (index, work_id) in topological_order_for_state(state)?.iter().enumerate() {
        let item = state.items.get(work_id).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("missing item state for work item: {work_id}"),
                state.loop_meta.id.clone(),
            )
        })?;
        let deps = state
            .dependencies
            .get(work_id)
            .filter(|deps| !deps.is_empty())
            .map(|deps| deps.join(","))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {}. {} status={} rounds={} depends_on={}",
            index + 1,
            work_id,
            item.status.as_str(),
            item.round_count,
            deps
        );
    }
    Ok(())
}
