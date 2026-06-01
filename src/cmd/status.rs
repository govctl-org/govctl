//! Status command implementation.

use crate::config::Config;
use crate::diagnostic::{DiagnosticResult, Diagnostics};
use crate::load::load_project;
use crate::model::{AdrStatus, ClauseStatus, RfcPhase, RfcStatus, WorkItemStatus};
use crate::theme::status_semantic;
use crate::ui::stdout_supports_color;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::hash::Hash;

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

fn count_by<I, T, K, F>(items: I, mut key: F) -> HashMap<K, usize>
where
    I: IntoIterator<Item = T>,
    K: Eq + Hash,
    F: FnMut(T) -> K,
{
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(key(item)).or_insert(0) += 1;
    }
    counts
}

fn count_for<K: Eq + Hash>(counts: &HashMap<K, usize>, key: K) -> usize {
    counts.get(&key).copied().unwrap_or(0)
}

fn total_count<K>(counts: &HashMap<K, usize>) -> usize {
    counts.values().sum()
}

fn status_breakdown<K>(counts: &HashMap<K, usize>, rows: &[(&str, K, &str)])
where
    K: Copy + Eq + Hash,
{
    for &(label, key, semantic) in rows {
        status_line(label, count_for(counts, key), semantic);
    }
}

/// Show summary status
pub fn show_status(config: &Config) -> DiagnosticResult<Diagnostics> {
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

    let by_status = count_by(&index.rfcs, |rfc| rfc.rfc.status);
    let by_phase = count_by(&index.rfcs, |rfc| rfc.rfc.phase);

    status_breakdown(
        &by_status,
        &[
            ("draft", RfcStatus::Draft, "draft"),
            ("normative", RfcStatus::Normative, "normative"),
            ("deprecated", RfcStatus::Deprecated, "deprecated"),
        ],
    );

    // Show phase breakdown for non-stable RFCs
    let spec = count_for(&by_phase, RfcPhase::Spec);
    let impl_phase = count_for(&by_phase, RfcPhase::Impl);
    let test = count_for(&by_phase, RfcPhase::Test);
    let stable = count_for(&by_phase, RfcPhase::Stable);

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

    let clause_by_status = count_by(index.iter_clauses(), |(_, clause)| clause.spec.status);
    status_breakdown(
        &clause_by_status,
        &[
            ("active", ClauseStatus::Active, "active"),
            ("deprecated", ClauseStatus::Deprecated, "deprecated"),
            ("superseded", ClauseStatus::Superseded, "superseded"),
        ],
    );
    total_line(total_count(&clause_by_status));

    // ADR summary
    section_header("ADRs");

    let adr_by_status = count_by(&index.adrs, |adr| adr.meta().status);
    status_breakdown(
        &adr_by_status,
        &[
            ("proposed", AdrStatus::Proposed, "proposed"),
            ("accepted", AdrStatus::Accepted, "accepted"),
            ("superseded", AdrStatus::Superseded, "superseded"),
        ],
    );
    total_line(index.adrs.len());

    // Work Item summary
    section_header("Work Items");

    let work_by_status = count_by(&index.work_items, |item| item.meta().status);
    status_breakdown(
        &work_by_status,
        &[
            ("queue", WorkItemStatus::Queue, "queue"),
            ("active", WorkItemStatus::Active, "active"),
            ("done", WorkItemStatus::Done, "done"),
            ("cancelled", WorkItemStatus::Cancelled, "cancelled"),
        ],
    );
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
