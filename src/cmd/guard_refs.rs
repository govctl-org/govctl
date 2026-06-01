use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::GuardEntry;
use crate::parse::{load_guards, load_work_items};

pub(crate) fn load_guard_by_id(config: &Config, id: &str) -> DiagnosticResult<GuardEntry> {
    load_guards(config)?
        .into_iter()
        .find(|guard| guard.spec.govctl.id == id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1002GuardNotFound,
                format!("Guard not found: {id}"),
                id,
            )
        })
}

pub(crate) fn guard_reference_blockers(
    config: &Config,
    guard_id: &str,
) -> DiagnosticResult<Vec<String>> {
    let mut blockers = Vec::new();

    if config
        .verification
        .default_guards
        .iter()
        .any(|id| id == guard_id)
    {
        blockers.push("Listed in verification.default_guards in gov/config.toml".to_string());
    }

    for work_item in &load_work_items(config)? {
        if work_item
            .spec
            .verification
            .required_guards
            .iter()
            .any(|id| id == guard_id)
        {
            blockers.push(format!(
                "Referenced by work item {}",
                work_item.spec.govctl.id
            ));
        }
        for waiver in &work_item.spec.verification.waivers {
            if waiver.guard == guard_id {
                blockers.push(format!("Waiver in work item {}", work_item.spec.govctl.id));
            }
        }
    }

    Ok(blockers)
}
