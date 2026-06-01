use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use std::collections::BTreeSet;

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
