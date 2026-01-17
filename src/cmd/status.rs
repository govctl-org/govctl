//! Status command implementation.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::model::{AdrStatus, ClauseStatus, RfcPhase, RfcStatus, WorkItemStatus};
use std::collections::HashMap;

/// Show summary status
pub fn show_status(config: &Config) -> anyhow::Result<Vec<Diagnostic>> {
    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };

    // RFC summary
    println!("=== RFCs ===\n");

    let mut by_status: HashMap<RfcStatus, usize> = HashMap::new();
    let mut by_phase: HashMap<RfcPhase, usize> = HashMap::new();

    for rfc in &index.rfcs {
        *by_status.entry(rfc.rfc.status).or_insert(0) += 1;
        *by_phase.entry(rfc.rfc.phase).or_insert(0) += 1;
    }

    println!("  By Status:");
    for status in [
        RfcStatus::Draft,
        RfcStatus::Normative,
        RfcStatus::Deprecated,
    ] {
        let count = by_status.get(&status).copied().unwrap_or(0);
        if count > 0 {
            println!("    {:12}: {}", status.as_ref(), count);
        }
    }

    println!("  By Phase:");
    for phase in [
        RfcPhase::Spec,
        RfcPhase::Impl,
        RfcPhase::Test,
        RfcPhase::Stable,
    ] {
        let count = by_phase.get(&phase).copied().unwrap_or(0);
        if count > 0 {
            println!("    {:12}: {}", phase.as_ref(), count);
        }
    }

    println!("  ----------");
    println!("  Total:        {}\n", index.rfcs.len());

    // Clause summary
    println!("=== Clauses ===\n");

    let mut clause_by_status: HashMap<ClauseStatus, usize> = HashMap::new();
    let mut total_clauses = 0;

    for (_, clause) in index.iter_clauses() {
        *clause_by_status.entry(clause.spec.status).or_insert(0) += 1;
        total_clauses += 1;
    }

    for status in [
        ClauseStatus::Active,
        ClauseStatus::Deprecated,
        ClauseStatus::Superseded,
    ] {
        let count = clause_by_status.get(&status).copied().unwrap_or(0);
        if count > 0 {
            println!("    {:12}: {}", status.as_ref(), count);
        }
    }

    println!("  ----------");
    println!("  Total:        {}\n", total_clauses);

    // ADR summary
    println!("=== ADRs ===\n");

    let mut adr_by_status: HashMap<AdrStatus, usize> = HashMap::new();

    for adr in &index.adrs {
        *adr_by_status.entry(adr.meta().status).or_insert(0) += 1;
    }

    for status in [AdrStatus::Proposed, AdrStatus::Accepted, AdrStatus::Superseded] {
        let count = adr_by_status.get(&status).copied().unwrap_or(0);
        if count > 0 {
            println!("    {:12}: {}", status.as_ref(), count);
        }
    }

    println!("  ----------");
    println!("  Total:        {}\n", index.adrs.len());

    // Work Item summary
    println!("=== Work Items ===\n");

    let mut work_by_status: HashMap<WorkItemStatus, usize> = HashMap::new();

    for item in &index.work_items {
        *work_by_status.entry(item.meta().status).or_insert(0) += 1;
    }

    for status in [
        WorkItemStatus::Queue,
        WorkItemStatus::Active,
        WorkItemStatus::Done,
        WorkItemStatus::Cancelled,
    ] {
        let count = work_by_status.get(&status).copied().unwrap_or(0);
        if count > 0 {
            println!("    {:12}: {}", status.as_ref(), count);
        }
    }

    println!("  ----------");
    println!("  Total:        {}", index.work_items.len());

    Ok(vec![])
}
