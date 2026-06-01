//! List command implementation.

mod output;
mod resources;
mod summaries;

use crate::ListTarget;
use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::parse::load_guards_with_warnings;
use resources::{list_adrs, list_clauses, list_guards, list_rfcs, list_work_items};

/// List artifacts
pub fn list(
    config: &Config,
    target: ListTarget,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) -> anyhow::Result<Vec<Diagnostic>> {
    if target == ListTarget::Guard {
        let result = load_guards_with_warnings(config)?;
        list_guards(&result.items, filter, limit, output, tags);
        return Ok(result.warnings);
    }

    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };

    match target {
        ListTarget::Rfc => list_rfcs(&index, filter, limit, output, tags),
        ListTarget::Clause => list_clauses(&index, filter, limit, output, tags),
        ListTarget::Adr => list_adrs(&index, filter, limit, output, tags),
        ListTarget::Work => list_work_items(&index, filter, limit, output, tags),
        ListTarget::Guard => unreachable!("handled above"),
    }

    Ok(vec![])
}
