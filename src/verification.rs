//! Shared verification guard loading, validation, and execution.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{GuardEntry, WorkItemEntry};
use regex::RegexBuilder;
use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_GUARD_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct GuardRunResult {
    pub id: String,
    pub passed: bool,
    pub timed_out: bool,
    pub output: String,
}

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

pub fn run_guard(config: &Config, guard: &GuardEntry) -> Result<GuardRunResult, Diagnostic> {
    let project_root = config
        .gov_root
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| config.gov_root.clone());
    let timeout = if guard.spec.check.timeout_secs == 0 {
        DEFAULT_GUARD_TIMEOUT_SECS
    } else {
        guard.spec.check.timeout_secs
    };

    let mut child = Command::new("/bin/bash")
        .args(["-lc", &guard.spec.check.command])
        .current_dir(project_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to start verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })?;

    let deadline = Duration::from_secs(timeout);
    let started = Instant::now();
    let mut timed_out = false;

    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < deadline => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Ok(None) => {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            Err(err) => {
                return Err(Diagnostic::new(
                    DiagnosticCode::E1004GuardCheckFailed,
                    format!(
                        "Failed while waiting on verification guard '{}': {}",
                        guard.meta().id,
                        err
                    ),
                    guard.path.display().to_string(),
                ));
            }
        }
    }

    let output = child.wait_with_output().map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E1004GuardCheckFailed,
            format!(
                "Failed to collect output for verification guard '{}': {}",
                guard.meta().id,
                err
            ),
            guard.path.display().to_string(),
        )
    })?;
    let combined_output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let pattern_matched = match &guard.spec.check.pattern {
        Some(pattern) => RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .map_err(|err| {
                Diagnostic::new(
                    DiagnosticCode::E1001GuardSchemaInvalid,
                    format!(
                        "Invalid regex pattern for guard '{}': {}",
                        guard.meta().id,
                        err
                    ),
                    guard.path.display().to_string(),
                )
            })?
            .is_match(&combined_output),
        None => true,
    };

    Ok(GuardRunResult {
        id: guard.meta().id.clone(),
        passed: !timed_out && output.status.success() && pattern_matched,
        timed_out,
        output: combined_output,
    })
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
