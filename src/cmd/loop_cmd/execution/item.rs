use super::super::state::loop_item_state;
use crate::cmd::work_lookup::load_work_item_by_id;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult};
use crate::loop_state::{LoopRoundRecord, LoopState, LoopWorkItemStatus, write_loop_round_record};
use crate::model::{ChecklistStatus, WorkItemEntry, WorkItemStatus};
use crate::write::WriteOp;
use std::path::Path;

pub(super) fn execute_work_item_round(
    config: &Config,
    state: &mut LoopState,
    work_id: &str,
    max_rounds: u32,
    op: WriteOp,
) -> DiagnosticResult<Option<String>> {
    let item_status_before = loop_item_status_string(state, work_id)?;
    let mut entry = load_work_item_by_id(config, work_id)?;
    let work_status_before = work_item_status_string(entry.spec.govctl.status);
    match entry.spec.govctl.status {
        WorkItemStatus::Done => {
            state.set_item_status(work_id, LoopWorkItemStatus::Done)?;
            return Ok(None);
        }
        WorkItemStatus::Cancelled => {
            state.set_item_status(work_id, LoopWorkItemStatus::Cancelled)?;
            return Ok(None);
        }
        WorkItemStatus::Queue => {
            if let Err(err) =
                crate::cmd::move_::move_item(config, Path::new(work_id), WorkItemStatus::Active, op)
            {
                state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
                return Ok(Some(format!(
                    "{work_id}: failed to transition to active: {}",
                    error_summary(&err)
                )));
            }
            if !op.is_preview() {
                entry = load_work_item_by_id(config, work_id)?;
            } else {
                entry.spec.govctl.status = WorkItemStatus::Active;
            }
        }
        WorkItemStatus::Active => {}
    }

    state.set_item_status(work_id, LoopWorkItemStatus::Active)?;
    let current_rounds = loop_item_state(state, work_id)?.round_count;
    if current_rounds >= max_rounds {
        state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
        return Ok(Some(format!(
            "{work_id}: maximum rounds reached ({max_rounds})"
        )));
    }
    let round = state.increment_round_count(work_id)?;

    let (action, reason, failure) = if acceptance_criteria_satisfied(&entry) {
        match crate::cmd::move_::move_item(config, Path::new(work_id), WorkItemStatus::Done, op) {
            Ok(_) => {
                state.set_item_status(work_id, LoopWorkItemStatus::Done)?;
                entry.spec.govctl.status = WorkItemStatus::Done;
                (
                    "evaluated acceptance criteria and completed work item".to_string(),
                    None,
                    None,
                )
            }
            Err(err) => {
                let summary = error_summary(&err);
                if round < max_rounds && is_retryable_guard_assertion_failure(&summary) {
                    (
                        "evaluated acceptance criteria and verification guards".to_string(),
                        Some(format!(
                            "verification guard assertion failed; max rounds not reached: {summary}"
                        )),
                        None,
                    )
                } else {
                    state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
                    let reason = format!("failed to complete after round {round}: {summary}");
                    (
                        "evaluated acceptance criteria and verification guards".to_string(),
                        Some(reason.clone()),
                        Some(format!("{work_id}: {reason}")),
                    )
                }
            }
        }
    } else if round >= max_rounds {
        state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
        let reason =
            format!("maximum rounds reached ({max_rounds}) with pending acceptance criteria");
        (
            "evaluated acceptance criteria".to_string(),
            Some(reason.clone()),
            Some(format!("{work_id}: {reason}")),
        )
    } else {
        (
            "evaluated acceptance criteria".to_string(),
            Some("pending acceptance criteria remain; max rounds not reached".to_string()),
            None,
        )
    };

    let item_status_after = loop_item_status_string(state, work_id)?;
    let work_status_after = if op.is_preview() {
        work_item_status_string(entry.spec.govctl.status)
    } else {
        work_item_status_string(load_work_item_by_id(config, work_id)?.spec.govctl.status)
    };
    write_loop_round_record(
        config,
        &LoopRoundRecord {
            loop_id: state.loop_meta.id.clone(),
            work_item_id: work_id.to_string(),
            round_number: round,
            max_rounds,
            item_status_before,
            item_status_after: item_status_after.clone(),
            work_status_before,
            work_status_after,
            action,
            outcome: item_status_after,
            reason,
        },
        op,
    )?;
    Ok(failure)
}

fn acceptance_criteria_satisfied(entry: &WorkItemEntry) -> bool {
    !entry.spec.content.acceptance_criteria.is_empty()
        && entry
            .spec
            .content
            .acceptance_criteria
            .iter()
            .all(|criterion| criterion.status != ChecklistStatus::Pending)
}

fn loop_item_status_string(state: &LoopState, work_id: &str) -> DiagnosticResult<String> {
    Ok(loop_item_state(state, work_id)?.status.as_str().to_string())
}

fn work_item_status_string(status: WorkItemStatus) -> String {
    match status {
        WorkItemStatus::Queue => "queue",
        WorkItemStatus::Active => "active",
        WorkItemStatus::Done => "done",
        WorkItemStatus::Cancelled => "cancelled",
    }
    .to_string()
}

fn error_summary(err: &Diagnostic) -> String {
    err.message.clone()
}

fn is_retryable_guard_assertion_failure(summary: &str) -> bool {
    summary.contains("verification guard requirements failed")
        && summary.contains("Verification guard '")
        && summary.contains(" failed")
        && !summary.contains("timed out")
        && !summary.contains("Failed to start")
        && !summary.contains("Unknown verification guard")
}
