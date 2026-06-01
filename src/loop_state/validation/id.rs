use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use chrono::NaiveDate;

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
