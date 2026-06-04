mod round;
mod state;

pub(super) use round::validate_loop_round_record;
pub(super) use state::validate_loop_state;

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_state::LoopLifecycleState;
use chrono::NaiveDate;
use std::collections::BTreeSet;

const LOOP_ID_FORMAT: &str = "LOOP-YYYY-MM-DD-NNN";

pub fn validate_loop_id(loop_id: &str) -> DiagnosticResult<()> {
    if !is_canonical_loop_id(loop_id) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1204LoopInvalidId,
            format!("Invalid loop ID '{loop_id}': must use canonical format {LOOP_ID_FORMAT}"),
            loop_id,
        ));
    }
    Ok(())
}

pub(in crate::loop_state) fn ensure_work_item_id(
    work_id: &str,
    loop_id: &str,
) -> DiagnosticResult<()> {
    if crate::validate::is_work_item_id(work_id) {
        Ok(())
    } else {
        Err(invalid_state(
            loop_id,
            format!("invalid work item ID in loop state: {work_id}"),
        ))
    }
}

pub(in crate::loop_state) fn invalid_state(
    loop_id: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E1201LoopStateInvalid, message, loop_id)
}

pub(super) fn ensure_no_duplicates(
    values: &[String],
    field: &str,
    loop_id: &str,
) -> DiagnosticResult<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.as_str()) {
            return Err(invalid_state(
                loop_id,
                format!("duplicate value '{value}' in {field}"),
            ));
        }
    }
    Ok(())
}

pub(in crate::loop_state) fn validate_loop_transition(
    loop_id: &str,
    from: LoopLifecycleState,
    to: LoopLifecycleState,
) -> DiagnosticResult<()> {
    if is_valid_loop_transition(from, to) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1203LoopInvalidTransition,
            format!("Invalid loop transition: {from:?} -> {to:?}"),
            loop_id,
        ))
    }
}

fn is_canonical_loop_id(loop_id: &str) -> bool {
    if loop_id.len() != "LOOP-YYYY-MM-DD-NNN".len() {
        return false;
    }
    if !loop_id.starts_with("LOOP-") {
        return false;
    }
    let bytes = loop_id.as_bytes();
    if bytes[9] != b'-' || bytes[12] != b'-' || bytes[15] != b'-' {
        return false;
    }
    if !bytes[5..9].iter().all(|byte| byte.is_ascii_digit())
        || !bytes[10..12].iter().all(|byte| byte.is_ascii_digit())
        || !bytes[13..15].iter().all(|byte| byte.is_ascii_digit())
        || !bytes[16..19].iter().all(|byte| byte.is_ascii_digit())
    {
        return false;
    }
    let date = &loop_id[5..15];
    if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return false;
    }
    &loop_id[16..19] != "000"
}

fn is_valid_loop_transition(from: LoopLifecycleState, to: LoopLifecycleState) -> bool {
    matches!(
        (from, to),
        (LoopLifecycleState::Pending, LoopLifecycleState::Active)
            | (LoopLifecycleState::Active, LoopLifecycleState::Paused)
            | (LoopLifecycleState::Paused, LoopLifecycleState::Active)
            | (LoopLifecycleState::Active, LoopLifecycleState::Completed)
            | (LoopLifecycleState::Active, LoopLifecycleState::Failed)
            | (LoopLifecycleState::Paused, LoopLifecycleState::Failed)
    )
}
