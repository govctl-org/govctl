use super::common::{ensure_work_item_id, invalid_state};
use super::id::validate_loop_id;
use crate::loop_state::LoopRoundRecord;

pub(in crate::loop_state) fn validate_loop_round_record(
    record: &LoopRoundRecord,
) -> anyhow::Result<()> {
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
