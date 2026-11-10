//! Record one Work Item verification waiver.
//!
//! Implements [[RFC-0002:C-WORK-WAIVE]].

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::GuardWaiver;
use crate::parse::{load_guards, write_work_item};
use crate::ui;
use crate::verification::{build_guard_index, required_guard_ids_before_waivers};
use crate::write::WriteOp;

pub fn waive_guard(
    config: &Config,
    work_id: &str,
    guard_id: &str,
    reason: Option<&str>,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let mut entry = crate::cmd::work_lookup::load_work_item_by_id(config, work_id)
        .map_err(|diag| rejection(diag.code, &diag.message, work_id, guard_id))?;
    let reason = reason.unwrap_or("").trim();
    if reason.is_empty() {
        return Err(rejection(
            DiagnosticCode::E0401WorkSchemaInvalid,
            &format!("Verification waiver reason must not be empty: {guard_id}"),
            work_id,
            guard_id,
        ));
    }
    if entry
        .spec
        .verification
        .waivers
        .iter()
        .any(|waiver| waiver.guard == guard_id)
    {
        return Err(rejection(
            DiagnosticCode::E0401WorkSchemaInvalid,
            &format!("Duplicate waiver for verification guard: {guard_id}"),
            work_id,
            guard_id,
        ));
    }

    let (guards, _) = build_guard_index(load_guards(config)?);
    if !guards.contains_key(guard_id) {
        return Err(rejection(
            DiagnosticCode::E1002GuardNotFound,
            &format!("Work item waiver references unknown verification guard: {guard_id}"),
            work_id,
            guard_id,
        ));
    }
    if !required_guard_ids_before_waivers(config, &entry)
        .iter()
        .any(|required_id| required_id == guard_id)
    {
        return Err(rejection(
            DiagnosticCode::E0401WorkSchemaInvalid,
            &format!(
                "Verification waiver references guard that is not required for this work item: {guard_id}"
            ),
            work_id,
            guard_id,
        ));
    }

    entry.spec.verification.waivers.push(GuardWaiver {
        guard: guard_id.to_string(),
        reason: reason.to_string(),
    });
    write_work_item(
        &entry.path,
        &entry.spec,
        op,
        Some(&config.display_path(&entry.path)),
    )?;
    if op.is_preview() {
        ui::would_record_waiver(work_id, guard_id, reason);
    } else {
        ui::field_added(work_id, "verification.waivers", guard_id);
    }
    Ok(vec![])
}

fn rejection(code: DiagnosticCode, message: &str, work_id: &str, guard_id: &str) -> Diagnostic {
    let mut message = message.to_string();
    if !message_names_id(&message, guard_id) {
        message.push_str(&format!(" Guard: {guard_id}."));
    }
    Diagnostic::new(code, message, work_id)
}

fn message_names_id(message: &str, id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    message.match_indices(id).any(|(start, _)| {
        let before = message[..start].chars().next_back();
        let after = message[start + id.len()..].chars().next();
        boundary_ok(before) && boundary_ok(after)
    })
}

fn boundary_ok(ch: Option<char>) -> bool {
    match ch {
        Some(ch) => !ch.is_ascii_alphanumeric() && !matches!(ch, '-' | '_' | ':'),
        None => true,
    }
}
