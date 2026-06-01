use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::WorkItemEntry;
use crate::parse::load_work_items;

pub(crate) fn load_work_item_by_id(
    config: &Config,
    work_id: &str,
) -> DiagnosticResult<WorkItemEntry> {
    load_work_items(config)?
        .into_iter()
        .find(|item| item.spec.govctl.id == work_id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {work_id}"),
                work_id,
            )
        })
}
