use super::{ensure_no_duplicates, ensure_work_item_id, invalid_state, validate_loop_id};
use crate::diagnostic::DiagnosticResult;
use crate::loop_state::LoopState;
use std::collections::BTreeSet;

pub(in crate::loop_state) fn validate_loop_state(
    state: &LoopState,
    expected_loop_id: Option<&str>,
) -> DiagnosticResult<()> {
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

    ensure_no_duplicates(&state.loop_meta.work, "loop.work", &state.loop_meta.id)?;
    ensure_no_duplicates(
        &state.loop_meta.resolved,
        "loop.resolved",
        &state.loop_meta.id,
    )?;

    let resolved_work_ids: BTreeSet<&str> = state
        .loop_meta
        .resolved
        .iter()
        .map(String::as_str)
        .collect();
    for work_id in &state.loop_meta.resolved {
        ensure_work_item_id(work_id, &state.loop_meta.id)?;
    }
    for root in &state.loop_meta.work {
        ensure_work_item_id(root, &state.loop_meta.id)?;
        if !resolved_work_ids.contains(root.as_str()) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("work item '{root}' is missing from loop.resolved"),
            ));
        }
    }

    for work_id in &state.loop_meta.resolved {
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
        if !resolved_work_ids.contains(work_id.as_str()) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("dependency entry '{work_id}' is not in loop.resolved"),
            ));
        }
        ensure_no_duplicates(
            dependencies,
            &format!("dependencies.{work_id}"),
            &state.loop_meta.id,
        )?;
        for dependency in dependencies {
            ensure_work_item_id(dependency, &state.loop_meta.id)?;
            if !resolved_work_ids.contains(dependency.as_str()) {
                return Err(invalid_state(
                    &state.loop_meta.id,
                    format!(
                        "dependency '{dependency}' for '{work_id}' is missing from loop.resolved"
                    ),
                ));
            }
        }
    }

    for work_id in state.items.keys() {
        if !resolved_work_ids.contains(work_id.as_str()) {
            return Err(invalid_state(
                &state.loop_meta.id,
                format!("item state '{work_id}' is not in loop.resolved"),
            ));
        }
    }

    Ok(())
}
