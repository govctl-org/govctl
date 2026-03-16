//! Status command implementation.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::model::{AdrStatus, ClauseStatus, RfcPhase, RfcStatus, WorkItemStatus};
use crate::theme::status_semantic;
use crate::ui::stdout_supports_color;
use owo_colors::OwoColorize;
use std::collections::HashMap;

fn use_colors() -> bool {
    stdout_supports_color()
}

fn section_header(title: &str) {
    if use_colors() {
        println!("\n{}", title.bold().underline());
    } else {
        println!("\n{}", title);
    }
}

fn status_line(label: &str, count: usize, status: &str) {
    if count == 0 {
        return;
    }

    if use_colors() {
        let color = status_semantic(status).to_owo();
        let count_str = count.to_string();
        println!(
            "  {:12} {}",
            label.color(color),
            count_str.color(color).bold()
        );
    } else {
        println!("  {:12} {}", label, count);
    }
}

fn total_line(count: usize) {
    if use_colors() {
        println!("  {:12} {}", "Total".dimmed(), count.to_string().bold());
    } else {
        println!("  {:12} {}", "Total", count);
    }
}

/// Show summary status
pub fn show_status(config: &Config) -> anyhow::Result<Vec<Diagnostic>> {
    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };

    // Header
    if use_colors() {
        println!("{}", "govctl status".bold());
    } else {
        println!("govctl status");
    }

    // RFC summary
    section_header("RFCs");

    let mut by_status: HashMap<RfcStatus, usize> = HashMap::new();
    let mut by_phase: HashMap<RfcPhase, usize> = HashMap::new();

    for rfc in &index.rfcs {
        *by_status.entry(rfc.rfc.status).or_insert(0) += 1;
        *by_phase.entry(rfc.rfc.phase).or_insert(0) += 1;
    }

    // Show status breakdown
    let draft = by_status.get(&RfcStatus::Draft).copied().unwrap_or(0);
    let normative = by_status.get(&RfcStatus::Normative).copied().unwrap_or(0);
    let deprecated = by_status.get(&RfcStatus::Deprecated).copied().unwrap_or(0);

    status_line("draft", draft, "draft");
    status_line("normative", normative, "normative");
    status_line("deprecated", deprecated, "deprecated");

    // Show phase breakdown for non-stable RFCs
    let spec = by_phase.get(&RfcPhase::Spec).copied().unwrap_or(0);
    let impl_phase = by_phase.get(&RfcPhase::Impl).copied().unwrap_or(0);
    let test = by_phase.get(&RfcPhase::Test).copied().unwrap_or(0);
    let stable = by_phase.get(&RfcPhase::Stable).copied().unwrap_or(0);

    if spec > 0 || impl_phase > 0 || test > 0 {
        println!();
        if use_colors() {
            print!("  {} ", "phases".dimmed());
            print!("spec:");
            if spec > 0 {
                print!("{} ", spec.to_string().yellow());
            } else {
                print!("{} ", "0".dimmed());
            }
            print!("impl:");
            if impl_phase > 0 {
                print!("{} ", impl_phase.to_string().yellow());
            } else {
                print!("{} ", "0".dimmed());
            }
            print!("test:");
            if test > 0 {
                print!("{} ", test.to_string().yellow());
            } else {
                print!("{} ", "0".dimmed());
            }
            print!("stable:");
            println!("{}", stable.to_string().green());
        } else {
            println!(
                "  phases spec:{} impl:{} test:{} stable:{}",
                spec, impl_phase, test, stable
            );
        }
    }

    total_line(index.rfcs.len());

    // Clause summary
    section_header("Clauses");

    let mut clause_by_status: HashMap<ClauseStatus, usize> = HashMap::new();
    let mut total_clauses = 0;

    for (_, clause) in index.iter_clauses() {
        *clause_by_status.entry(clause.spec.status).or_insert(0) += 1;
        total_clauses += 1;
    }

    let active = clause_by_status
        .get(&ClauseStatus::Active)
        .copied()
        .unwrap_or(0);
    let clause_deprecated = clause_by_status
        .get(&ClauseStatus::Deprecated)
        .copied()
        .unwrap_or(0);
    let superseded = clause_by_status
        .get(&ClauseStatus::Superseded)
        .copied()
        .unwrap_or(0);

    status_line("active", active, "active");
    status_line("deprecated", clause_deprecated, "deprecated");
    status_line("superseded", superseded, "superseded");
    total_line(total_clauses);

    // ADR summary
    section_header("ADRs");

    let mut adr_by_status: HashMap<AdrStatus, usize> = HashMap::new();

    for adr in &index.adrs {
        *adr_by_status.entry(adr.meta().status).or_insert(0) += 1;
    }

    let proposed = adr_by_status
        .get(&AdrStatus::Proposed)
        .copied()
        .unwrap_or(0);
    let accepted = adr_by_status
        .get(&AdrStatus::Accepted)
        .copied()
        .unwrap_or(0);
    let adr_superseded = adr_by_status
        .get(&AdrStatus::Superseded)
        .copied()
        .unwrap_or(0);

    status_line("proposed", proposed, "proposed");
    status_line("accepted", accepted, "accepted");
    status_line("superseded", adr_superseded, "superseded");
    total_line(index.adrs.len());

    // Work Item summary
    section_header("Work Items");

    let mut work_by_status: HashMap<WorkItemStatus, usize> = HashMap::new();

    for item in &index.work_items {
        *work_by_status.entry(item.meta().status).or_insert(0) += 1;
    }

    let queue = work_by_status
        .get(&WorkItemStatus::Queue)
        .copied()
        .unwrap_or(0);
    let work_active = work_by_status
        .get(&WorkItemStatus::Active)
        .copied()
        .unwrap_or(0);
    let done = work_by_status
        .get(&WorkItemStatus::Done)
        .copied()
        .unwrap_or(0);
    let cancelled = work_by_status
        .get(&WorkItemStatus::Cancelled)
        .copied()
        .unwrap_or(0);

    status_line("queue", queue, "queue");
    status_line("active", work_active, "active");
    status_line("done", done, "done");
    status_line("cancelled", cancelled, "cancelled");
    total_line(index.work_items.len());

    // Show active work items if any
    let active_items: Vec<_> = index
        .work_items
        .iter()
        .filter(|w| w.meta().status == WorkItemStatus::Active)
        .collect();

    if !active_items.is_empty() {
        println!();
        if use_colors() {
            println!("{}", "Active Work".bold().underline());
        } else {
            println!("Active Work");
        }
        for item in active_items {
            if use_colors() {
                println!("  {} {}", item.meta().id.cyan().bold(), item.meta().title);
            } else {
                println!("  {} {}", item.meta().id, item.meta().title);
            }
        }
    }

    println!();
    Ok(vec![])
}
