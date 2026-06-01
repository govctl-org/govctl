use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_rfcs;
use crate::parse::{load_adrs, load_guards_with_warnings, load_work_items};
use anyhow::Result;
use std::collections::HashMap;

/// Build a tag -> usage count map by loading all artifacts once.
pub(super) fn build_tag_usage_map(config: &Config) -> Result<HashMap<String, usize>> {
    let mut usage: HashMap<String, usize> = HashMap::new();

    let rfcs = load_rfcs(config).map_err(Diagnostic::from)?;
    for rfc_index in &rfcs {
        increment(&mut usage, &rfc_index.rfc.tags);
        for clause in &rfc_index.clauses {
            increment(&mut usage, &clause.spec.tags);
        }
    }

    let adrs = load_adrs(config)?;
    for adr in &adrs {
        increment(&mut usage, &adr.spec.govctl.tags);
    }

    let items = load_work_items(config)?;
    for item in &items {
        increment(&mut usage, &item.spec.govctl.tags);
    }

    let guard_result = load_guards_with_warnings(config)?;
    for guard in &guard_result.items {
        increment(&mut usage, &guard.spec.govctl.tags);
    }

    Ok(usage)
}

fn increment(map: &mut HashMap<String, usize>, tags: &[String]) {
    for tag in tags {
        *map.entry(tag.clone()).or_insert(0) += 1;
    }
}
