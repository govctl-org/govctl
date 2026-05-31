use super::{LoopLifecycleState, LoopRoundRecord, LoopState};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use std::collections::BTreeSet;

pub(super) fn validate_loop_state(
    state: &LoopState,
    expected_loop_id: Option<&str>,
) -> anyhow::Result<()> {
    validate_loop_id(&state.loop_meta.id)?;
    if let Some(expected) = expected_loop_id
        && state.loop_meta.id != expected
    {
        return Err(invalid_state(
            &state.loop_meta.id,
            format!(
                "loop.id '{}' does not match loop directory '{}'",
                state.loop_meta.id, expected
            ),
        ));
    }

    ensure_no_duplicates(
        &state.loop_meta.root_work_items,
        "loop.root_work_items",
        &state.loop_meta.id,
    )?;
    ensure_no_duplicates(
        &state.loop_meta.work_items,
        "loop.work_items",
        &state.loop_meta.id,
    )?;

    let work_items: BTreeSet<&str> = state
        .loop_meta
        .work_items
        .iter()
        .map(String::as_str)
        .collect();
    for work_id in &state.loop_meta.work_items {
        ensure_work_item_id(work_id, &state.loop_meta.id)?;
    }
    for root in &state.loop_meta.root_work_items {
        ensure_work_item_id(root, &state.loop_meta.id)?;
        if !work_items.contains(root.as_str()) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("root work item '{root}' is missing from loop.work_items"),
            ));
        }
    }

    for work_id in &state.loop_meta.work_items {
        if !state.dependencies.contains_key(work_id) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("missing dependency entry for work item: {work_id}"),
            ));
        }
        if !state.items.contains_key(work_id) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("missing item state for work item: {work_id}"),
            ));
        }
    }

    for (work_id, dependencies) in &state.dependencies {
        if !work_items.contains(work_id.as_str()) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("dependency entry '{work_id}' is not in loop.work_items"),
            ));
        }
        ensure_no_duplicates(
            dependencies,
            &format!("dependencies.{work_id}"),
            &state.loop_meta.id,
        )?;
        for dependency in dependencies {
            ensure_work_item_id(dependency, &state.loop_meta.id)?;
            if !work_items.contains(dependency.as_str()) {
                return Err(invalid_state(
                    &state.loop_meta.id,
                    format!(
                        "dependency '{dependency}' for '{work_id}' is missing from loop.work_items"
                    ),
                ));
            }
        }
    }

    for work_id in state.items.keys() {
        if !work_items.contains(work_id.as_str()) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("item state '{work_id}' is not in loop.work_items"),
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_loop_round_record(record: &LoopRoundRecord) -> anyhow::Result<()> {
    validate_loop_id(&record.loop_id)?;
    ensure_work_item_id(&record.work_item_id, &record.loop_id)?;
    if record.round_number == 0 {
        return Err(invalid_state(
            &record.loop_id,
            "loop round record round_number must be at least 1",
        ));
    }
    if record.max_rounds == 0 {
        return Err(invalid_state(
            &record.loop_id,
            "loop round record max_rounds must be at least 1",
        ));
    }
    if record.round_number > record.max_rounds {
        return Err(invalid_state(
            &record.loop_id,
            format!(
                "loop round record round_number {} exceeds max_rounds {}",
                record.round_number, record.max_rounds
            ),
        ));
    }
    ensure_loop_item_status(
        &record.item_status_before,
        "item_status_before",
        &record.loop_id,
    )?;
    ensure_loop_item_status(
        &record.item_status_after,
        "item_status_after",
        &record.loop_id,
    )?;
    ensure_work_status(
        &record.work_status_before,
        "work_status_before",
        &record.loop_id,
    )?;
    ensure_work_status(
        &record.work_status_after,
        "work_status_after",
        &record.loop_id,
    )?;
    ensure_loop_item_status(&record.outcome, "outcome", &record.loop_id)?;
    ensure_non_empty(&record.action, "action", &record.loop_id)?;
    if let Some(reason) = &record.reason {
        ensure_non_empty(reason, "reason", &record.loop_id)?;
    }
    Ok(())
}

pub(super) fn validate_loop_transition(
    loop_id: &str,
    from: LoopLifecycleState,
    to: LoopLifecycleState,
) -> anyhow::Result<()> {
    if is_valid_loop_transition(from, to) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1203LoopInvalidTransition,
            format!("Invalid loop transition: {from:?} -> {to:?}"),
            loop_id,
        )
        .into())
    }
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

pub fn validate_loop_id(loop_id: &str) -> anyhow::Result<()> {
    let valid_first = loop_id
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric());
    let valid_rest = loop_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if loop_id.is_empty()
        || !valid_first
        || !valid_rest
        || loop_id.contains('/')
        || loop_id.contains('\\')
        || loop_id.contains("..")
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E1204LoopInvalidId,
            format!(
                "Invalid loop ID '{loop_id}': must match ^[A-Za-z0-9][A-Za-z0-9._-]*$ and must not contain path traversal"
            ),
            loop_id,
        )
        .into());
    }
    Ok(())
}

pub(super) fn ensure_work_item_id(work_id: &str, loop_id: &str) -> anyhow::Result<()> {
    if crate::validate::is_work_item_id(work_id) {
        Ok(())
    } else {
        Err(invalid_state(
            loop_id,
            format!("invalid work item ID in loop state: {work_id}"),
        ))
    }
}

pub(super) fn invalid_state(loop_id: &str, message: impl Into<String>) -> anyhow::Error {
    Diagnostic::new(DiagnosticCode::E1201LoopStateInvalid, message, loop_id).into()
}

fn ensure_no_duplicates(values: &[String], field: &str, loop_id: &str) -> anyhow::Result<()> {
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

fn ensure_loop_item_status(value: &str, field: &str, loop_id: &str) -> anyhow::Result<()> {
    if matches!(
        value,
        "pending" | "active" | "done" | "failed" | "blocked" | "cancelled"
    ) {
        Ok(())
    } else {
        Err(invalid_state(
            loop_id,
            format!("invalid loop round record {field}: {value}"),
        ))
    }
}

fn ensure_work_status(value: &str, field: &str, loop_id: &str) -> anyhow::Result<()> {
    if matches!(value, "queue" | "active" | "done" | "cancelled") {
        Ok(())
    } else {
        Err(invalid_state(
            loop_id,
            format!("invalid loop round record {field}: {value}"),
        ))
    }
}

fn ensure_non_empty(value: &str, field: &str, loop_id: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        Err(invalid_state(
            loop_id,
            format!("loop round record {field} must not be empty"),
        ))
    } else {
        Ok(())
    }
}
