use crate::config::Config;
use crate::diagnostic::DiagnosticResult;
use crate::model::WorkItemEntry;

pub(crate) fn load_work_item_by_id(
    config: &Config,
    work_id: &str,
) -> DiagnosticResult<WorkItemEntry> {
    crate::artifact_catalog::load_work_item_by_id(config, work_id)
}
