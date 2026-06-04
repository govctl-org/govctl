use super::{ensure_no_duplicates, ensure_work_item_id, invalid_state, validate_loop_id};
use crate::diagnostic::DiagnosticResult;
use crate::loop_state::{LoopRoundRecord, LoopRoundStatus};

pub(in crate::loop_state) fn validate_loop_round_record(
    record: &LoopRoundRecord,
) -> DiagnosticResult<()> {
    let round = &record.round_meta;
    validate_loop_id(&round.loop_id)?;
    if round.round_number == 0 {
        return Err(invalid_state(
            &round.loop_id,
            "loop round record round_number must be at least 1",
        ));
    }
    if round.max_rounds == 0 {
        return Err(invalid_state(
            &round.loop_id,
            "loop round record max_rounds must be at least 1",
        ));
    }
    if round.work.is_empty() {
        return Err(invalid_state(
            &round.loop_id,
            "loop round record work must not be empty",
        ));
    }
    ensure_no_duplicates(&round.work, "round.work", &round.loop_id)?;
    for work_id in &round.work {
        ensure_work_item_id(work_id, &round.loop_id)?;
    }
    ensure_non_empty_values(&record.summary.actions, "summary.actions", &round.loop_id)?;
    ensure_non_empty_values(
        &record.summary.changed_paths,
        "summary.changed_paths",
        &round.loop_id,
    )?;
    ensure_non_empty_values(
        &record.summary.verification,
        "summary.verification",
        &round.loop_id,
    )?;
    ensure_non_empty_values(&record.summary.blockers, "summary.blockers", &round.loop_id)?;
    ensure_non_empty_values(
        &record.summary.note_candidates,
        "summary.note_candidates",
        &round.loop_id,
    )?;
    if round.status == LoopRoundStatus::Closed && !record.has_required_summary_evidence() {
        return Err(invalid_state(
            &round.loop_id,
            "closed loop round record must include actions, changed_paths, and verification or blockers",
        ));
    }
    Ok(())
}

fn ensure_non_empty_values(values: &[String], field: &str, loop_id: &str) -> DiagnosticResult<()> {
    for value in values {
        if value.trim().is_empty() {
            return Err(invalid_state(
                loop_id,
                format!("loop round record {field} must not contain empty values"),
            ));
        }
    }
    Ok(())
}
