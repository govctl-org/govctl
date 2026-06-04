//! Shared verification guard loading, validation, and execution.

mod runner;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{GuardEntry, WorkItemEntry};
use regex::RegexBuilder;
use std::collections::{HashMap, HashSet};

pub use runner::run_guard;

pub fn build_guard_index(
    guards: Vec<GuardEntry>,
) -> (HashMap<String, GuardEntry>, Vec<Diagnostic>) {
    let mut index = HashMap::new();
    let mut diagnostics = Vec::new();

    for guard in guards {
        let guard_id = guard.meta().id.clone();
        if let Some(existing) = index.insert(guard_id.clone(), guard.clone()) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1003GuardDuplicate,
                format!(
                    "Duplicate verification guard ID '{}': {} and {}",
                    guard_id,
                    existing.path.display(),
                    guard.path.display()
                ),
                guard.path.display().to_string(),
            ));
        }
    }

    (index, diagnostics)
}

pub fn configured_default_guard_ids(config: &Config) -> Vec<String> {
    if config.verification.enabled {
        dedup_guard_ids(config.verification.default_guards.iter().cloned())
    } else {
        vec![]
    }
}

pub fn required_guard_ids_before_waivers(
    config: &Config,
    work_item: &WorkItemEntry,
) -> Vec<String> {
    dedup_guard_ids(
        configured_default_guard_ids(config)
            .into_iter()
            .chain(work_item.spec.verification.required_guards.iter().cloned()),
    )
}

pub fn effective_required_guard_ids(config: &Config, work_item: &WorkItemEntry) -> Vec<String> {
    let waived: HashSet<&str> = work_item
        .spec
        .verification
        .waivers
        .iter()
        .map(|waiver| waiver.guard.as_str())
        .collect();

    required_guard_ids_before_waivers(config, work_item)
        .into_iter()
        .filter(|guard_id| !waived.contains(guard_id.as_str()))
        .collect()
}

pub fn validate_guard_configuration(
    config: &Config,
    guards: &HashMap<String, GuardEntry>,
    work_items: &[WorkItemEntry],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(validate_verification_config(config, guards));
    for work_item in work_items {
        diagnostics.extend(validate_work_item_verification(config, guards, work_item));
    }
    diagnostics
}

pub fn validate_verification_config(
    config: &Config,
    guards: &HashMap<String, GuardEntry>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for guard_id in &config.verification.default_guards {
        if !guards.contains_key(guard_id) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("verification.default_guards references unknown guard: {guard_id}"),
                "gov/config.toml",
            ));
        }
    }

    for guard in guards.values() {
        if let Some(pattern) = &guard.spec.check.pattern
            && RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .is_err()
        {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1001GuardSchemaInvalid,
                format!("Invalid guard regex pattern: {}", guard.meta().id),
                guard.path.display().to_string(),
            ));
        }
    }

    diagnostics
}

pub fn validate_work_item_verification(
    config: &Config,
    guards: &HashMap<String, GuardEntry>,
    work_item: &WorkItemEntry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for guard_id in &work_item.spec.verification.required_guards {
        if !guards.contains_key(guard_id) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1002GuardNotFound,
                format!("Work item references unknown verification guard: {guard_id}"),
                config.display_path(&work_item.path).display().to_string(),
            ));
        }
    }

    let effective_before_waivers: HashSet<String> =
        required_guard_ids_before_waivers(config, work_item)
            .into_iter()
            .collect();
    let mut seen_waivers = HashSet::new();
    for waiver in &work_item.spec.verification.waivers {
        let work_path = config.display_path(&work_item.path).display().to_string();
        if !seen_waivers.insert(waiver.guard.as_str()) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!("Duplicate waiver for verification guard: {}", waiver.guard),
                work_path.clone(),
            ));
        }
        if waiver.reason.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!(
                    "Verification waiver reason must not be empty: {}",
                    waiver.guard
                ),
                work_path.clone(),
            ));
        }
        if !guards.contains_key(&waiver.guard) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1002GuardNotFound,
                format!(
                    "Work item waiver references unknown verification guard: {}",
                    waiver.guard
                ),
                work_path.clone(),
            ));
        }
        if !effective_before_waivers.contains(&waiver.guard) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!(
                    "Verification waiver references guard that is not required for this work item: {}",
                    waiver.guard
                ),
                work_path,
            ));
        }
    }

    diagnostics
}

pub fn unknown_guard_diagnostic(guard_id: &str, location: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1002GuardNotFound,
        format!("Unknown verification guard: {guard_id}"),
        location,
    )
}

fn dedup_guard_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for id in ids {
        if seen.insert(id.clone()) {
            deduped.push(id);
        }
    }

    deduped
}
