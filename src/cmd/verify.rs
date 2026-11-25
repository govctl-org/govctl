//! Verification guard command and work-item enforcement.

use crate::config::Config;
use crate::diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticResult, Diagnostics,
};
use crate::parse::{load_guard, load_guards_with_warnings};
use crate::ui;
use crate::verification;
use crate::write::WriteOp;
use std::collections::HashMap;
use std::path::Path;

use super::work_lookup::load_work_item_by_id;

pub fn verify(
    config: &Config,
    guard_ids: &[String],
    work_id: Option<&str>,
    record: Option<&Path>,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if let Some(record_path) = record {
        let Some(work_id) = work_id else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0822UnsupportedOperation,
                "A pass record requires --work",
                "verify",
            ));
        };
        return verify_pass_record(config, work_id, record_path, op);
    }

    let (guards_by_id, mut diagnostics) = load_guard_context(config)?;
    let work_item = if let Some(work_id) = work_id {
        Some(load_work_item_by_id(config, work_id)?)
    } else {
        None
    };

    if let Some(work_item) = work_item.as_ref() {
        diagnostics.extend(verification::validate_work_item_verification(
            config,
            &guards_by_id,
            work_item,
        ));
    }

    if diagnostics
        .iter()
        .any(|diag| diag.level == DiagnosticLevel::Error)
    {
        return Ok(diagnostics);
    }

    let selected_guard_ids = if let Some(work_item) = work_item.as_ref() {
        verification::effective_required_guard_ids(config, work_item)
    } else if !guard_ids.is_empty() {
        guard_ids.to_vec()
    } else {
        verification::configured_default_guard_ids(config)
    };

    if selected_guard_ids.is_empty() {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "No verification guards selected",
            work_id.unwrap_or("gov/config.toml"),
        ));
        return Ok(diagnostics);
    }

    diagnostics.extend(run_selected_guards(
        config,
        &guards_by_id,
        &selected_guard_ids,
        work_id.unwrap_or("verify"),
    )?);
    Ok(diagnostics)
}

/// Write a pass record for one work item. [[RFC-0002:C-PASS-RECORD-COMMAND]]
fn verify_pass_record(
    config: &Config,
    work_id: &str,
    record_path: &Path,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let work_item = load_work_item_by_id(config, work_id)?;
    let work_id = work_item.spec.govctl.id.as_str();

    let checkout = match super::pass_record::sample_checkout(config.project_root()) {
        Ok(checkout) => checkout,
        Err(err) => return Ok(checkout_diagnostics(err, work_id)),
    };
    let revision = super::pass_record::current_revision(&checkout).to_string();
    match super::pass_record::matches_revision(&checkout, &revision, Some(record_path)) {
        Ok(true) => {}
        Ok(false) => {
            return Ok(vec![super::pass_record::reject(
                "Checkout does not match the pass record revision",
                work_id,
            )]);
        }
        Err(err) => return Ok(checkout_diagnostics(err, work_id)),
    }

    let effective = verification::effective_required_guard_ids(config, &work_item);
    if effective.is_empty() {
        return Ok(vec![super::pass_record::empty_guard_set(work_id)]);
    }

    if op.is_preview() {
        // The report names the work item, revision, and guard set. It does not
        // claim that the guards passed.
        ui::info(format!(
            "Pass record plan for {work_id}: revision {revision}; guards {}",
            effective.join(", ")
        ));
        return Ok(vec![]);
    }

    let (guards_by_id, mut diagnostics) = load_guard_context(config)?;
    diagnostics.extend(verification::validate_work_item_verification(
        config,
        &guards_by_id,
        &work_item,
    ));
    if diagnostics
        .iter()
        .any(|diag| diag.level == DiagnosticLevel::Error)
    {
        return Ok(diagnostics);
    }

    diagnostics.extend(run_selected_guards(
        config,
        &guards_by_id,
        &effective,
        work_id,
    )?);
    if diagnostics
        .iter()
        .any(|diag| diag.level == DiagnosticLevel::Error)
    {
        return Ok(diagnostics);
    }

    match super::pass_record::matches_revision(&checkout, &revision, Some(record_path)) {
        Ok(true) => {}
        Ok(false) => {
            diagnostics.push(super::pass_record::reject(
                "Checkout does not match the pass record revision",
                work_id,
            ));
            return Ok(diagnostics);
        }
        Err(err) => {
            diagnostics.extend(checkout_diagnostics(err, work_id));
            return Ok(diagnostics);
        }
    }

    super::pass_record::write_record(record_path, work_id, &revision, &effective)?;
    Ok(diagnostics)
}

fn checkout_diagnostics(err: super::pass_record::InspectError, work_id: &str) -> Diagnostics {
    let message = "This checkout has no usable commit identifier";
    match err {
        super::pass_record::InspectError::NoVcs => {
            vec![super::pass_record::reject(message, work_id)]
        }
        super::pass_record::InspectError::Unreadable => vec![
            super::pass_record::unreadable_warning(work_id),
            super::pass_record::reject(message, work_id),
        ],
    }
}

/// Reject a done transition whose guard reference is unusable.
///
/// [[RFC-0001:C-GATE-CONDITIONS]]
pub fn reject_unusable_guard_reference(
    config: &Config,
    work_item: &crate::model::WorkItemEntry,
) -> DiagnosticResult<()> {
    load_guard_gate(config, work_item).map(|_| ())
}

/// Report the guards a `done` transition would run. Does not execute them.
///
/// [[RFC-0002:C-LIFECYCLE-VERBS]]
pub fn preview_work_item_guards(
    config: &Config,
    work_item: &crate::model::WorkItemEntry,
) -> DiagnosticResult<()> {
    // A guard file that does not parse is skipped. A duplicate ID is the same
    // error the real gate reports before it runs anything.
    let (guards_by_id, duplicate_diagnostics) =
        verification::build_guard_index(load_parsed_guards(config)?);
    let diagnostics =
        verification::validate_work_item_verification(config, &guards_by_id, work_item);
    let mut errors = error_lines(duplicate_diagnostics);
    errors.extend(error_lines(diagnostics));
    let required = &work_item.spec.verification.required_guards;
    let selected_guard_ids = verification::effective_required_guard_ids(config, work_item);
    for guard_id in &selected_guard_ids {
        let missing = !guards_by_id.contains_key(guard_id);
        // A missing required guard is already in `errors`.
        let reported = required.iter().any(|id| id == guard_id);
        if missing && !reported {
            errors.push(
                verification::unknown_guard_diagnostic(guard_id, &work_item.spec.govctl.id)
                    .to_string(),
            );
        }
    }
    if !errors.is_empty() {
        return Err(guard_requirement_failure(
            &work_item.spec.govctl.id,
            &errors,
        ));
    }

    let listed = if selected_guard_ids.is_empty() {
        "none".to_string()
    } else {
        selected_guard_ids.join(", ")
    };
    ui::info(format!(
        "Done plan for {}: guards {listed}",
        work_item.spec.govctl.id
    ));
    Ok(())
}

fn load_parsed_guards(config: &Config) -> DiagnosticResult<Vec<crate::model::GuardEntry>> {
    let mut guards = Vec::new();
    let guard_dir = config.guard_dir();
    if !guard_dir.exists() {
        return Ok(guards);
    }
    let entries = std::fs::read_dir(&guard_dir)
        .map_err(|err| Diagnostic::io_error("read guards", err, guard_dir.display().to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::io_error("read guards", err, guard_dir.display().to_string())
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        if let Ok(guard) = load_guard(config, &path) {
            guards.push(guard);
        }
    }
    Ok(guards)
}

pub fn enforce_work_item_guards(
    config: &Config,
    work_item: &crate::model::WorkItemEntry,
) -> DiagnosticResult<()> {
    let guards_by_id = load_guard_gate(config, work_item)?;
    let selected_guard_ids = verification::effective_required_guard_ids(config, work_item);
    if selected_guard_ids.is_empty() {
        return Ok(());
    }

    let run_diags = run_selected_guards(
        config,
        &guards_by_id,
        &selected_guard_ids,
        &work_item.spec.govctl.id,
    )?;
    let errors: Vec<String> = run_diags
        .into_iter()
        .filter(|diag| diag.level == DiagnosticLevel::Error)
        .map(|diag| diag.message)
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(guard_requirement_failure(
            &work_item.spec.govctl.id,
            &errors,
        ))
    }
}

fn load_guard_gate(
    config: &Config,
    work_item: &crate::model::WorkItemEntry,
) -> DiagnosticResult<HashMap<String, crate::model::GuardEntry>> {
    let (guards_by_id, mut diagnostics) = load_guard_context(config)?;
    diagnostics.extend(verification::validate_work_item_verification(
        config,
        &guards_by_id,
        work_item,
    ));
    let errors = error_lines(diagnostics);
    if errors.is_empty() {
        Ok(guards_by_id)
    } else {
        Err(guard_requirement_failure(
            &work_item.spec.govctl.id,
            &errors,
        ))
    }
}

fn error_lines(diagnostics: impl IntoIterator<Item = Diagnostic>) -> Vec<String> {
    diagnostics
        .into_iter()
        .filter(|diag| diag.level == DiagnosticLevel::Error)
        .map(|diag| diag.to_string())
        .collect()
}

fn guard_requirement_failure(work_id: &str, errors: &[String]) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1004GuardCheckFailed,
        format!(
            "Cannot mark as done: verification guard requirements failed:\n{}",
            errors
                .iter()
                .map(|msg| format!("  - {msg}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        work_id,
    )
}

fn load_guard_context(
    config: &Config,
) -> DiagnosticResult<(HashMap<String, crate::model::GuardEntry>, Diagnostics)> {
    let guard_result = load_guards_with_warnings(config)?;
    let mut diagnostics = guard_result.warnings;
    let (guards_by_id, guard_diags) = verification::build_guard_index(guard_result.items);
    diagnostics.extend(guard_diags);
    diagnostics.extend(verification::validate_verification_config(
        config,
        &guards_by_id,
    ));

    Ok((guards_by_id, diagnostics))
}

fn run_selected_guards(
    config: &Config,
    guards_by_id: &std::collections::HashMap<String, crate::model::GuardEntry>,
    selected_guard_ids: &[String],
    location: &str,
) -> DiagnosticResult<Diagnostics> {
    let mut diagnostics = Vec::new();

    for guard_id in selected_guard_ids {
        let Some(guard) = guards_by_id.get(guard_id) else {
            diagnostics.push(verification::unknown_guard_diagnostic(guard_id, location));
            continue;
        };

        let result = verification::run_guard(config, guard)?;
        if result.passed {
            ui::info(format!("PASS {}", result.id));
            continue;
        }

        let message = if result.timed_out {
            let primary_shell_state = match result.primary_shell_running_at_timeout {
                Some(true) => "primary shell process was still running when timeout handling began",
                Some(false) => {
                    "primary shell process had already exited when timeout handling began"
                }
                None => "primary shell process state at timeout was unavailable",
            };
            format!(
                "Verification guard '{}' timed out after {} seconds; {}",
                result.id, guard.spec.check.timeout_secs, primary_shell_state
            )
        } else {
            format!("Verification guard '{}' failed", result.id)
        };
        let code = if result.timed_out {
            DiagnosticCode::E1005GuardTimeout
        } else {
            DiagnosticCode::E1004GuardCheckFailed
        };
        let details = result.output.trim();
        diagnostics.push(Diagnostic::new(
            code,
            if details.is_empty() {
                message
            } else {
                format!("{message}: {details}")
            },
            guard.path.display().to_string(),
        ));
        ui::info(format!("FAIL {}", result.id));
    }

    Ok(diagnostics)
}
