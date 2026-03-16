//! Verification guard command and work-item enforcement.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLevel};
use crate::parse::{load_guards_with_warnings, load_work_items};
use crate::ui;
use crate::verification;
use std::collections::HashMap;

pub fn verify(
    config: &Config,
    guard_ids: &[String],
    work_id: Option<&str>,
) -> anyhow::Result<Vec<Diagnostic>> {
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

pub fn enforce_work_item_guards(
    config: &Config,
    work_item: &crate::model::WorkItemEntry,
) -> anyhow::Result<()> {
    let (guards_by_id, mut diagnostics) = load_guard_context(config)?;
    diagnostics.extend(verification::validate_work_item_verification(
        config,
        &guards_by_id,
        work_item,
    ));
    let mut errors: Vec<String> = diagnostics
        .into_iter()
        .filter(|diag| diag.level == DiagnosticLevel::Error)
        .map(|diag| diag.to_string())
        .collect();

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
    errors.extend(
        run_diags
            .into_iter()
            .filter(|diag| diag.level == DiagnosticLevel::Error)
            .map(|diag| diag.message),
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1004GuardCheckFailed,
            format!(
                "Cannot mark as done: verification guard requirements failed:\n{}",
                errors
                    .into_iter()
                    .map(|msg| format!("  - {msg}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            &work_item.spec.govctl.id,
        )
        .into())
    }
}

fn load_guard_context(
    config: &Config,
) -> anyhow::Result<(HashMap<String, crate::model::GuardEntry>, Vec<Diagnostic>)> {
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
) -> anyhow::Result<Vec<Diagnostic>> {
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
            format!(
                "Verification guard '{}' timed out after {} seconds",
                result.id, guard.spec.check.timeout_secs
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

fn load_work_item_by_id(
    config: &Config,
    work_id: &str,
) -> anyhow::Result<crate::model::WorkItemEntry> {
    load_work_items(config)?
        .into_iter()
        .find(|item| item.spec.govctl.id == work_id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {work_id}"),
                work_id,
            )
            .into()
        })
}
