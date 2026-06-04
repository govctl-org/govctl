use crate::config::Config;
use crate::diagnostic::DiagnosticResult;
use crate::model::ProjectIndex;

pub(super) fn clause_deletion_referrers(
    config: &Config,
    index: &ProjectIndex,
    clause_id: &str,
) -> DiagnosticResult<Vec<String>> {
    let mut referrers = project_referrers(index, clause_id, None, false);
    referrers.extend(guard_referrers(config, clause_id)?);
    referrers.sort();
    referrers.dedup();
    Ok(referrers)
}

pub(super) fn work_item_deletion_referrers(index: &ProjectIndex, work_id: &str) -> Vec<String> {
    project_referrers(index, work_id, Some(work_id), true)
}

fn project_referrers(
    index: &ProjectIndex,
    target_id: &str,
    skip_work_id: Option<&str>,
    include_work_dependencies: bool,
) -> Vec<String> {
    let mut referenced_by = Vec::new();

    for rfc in &index.rfcs {
        if rfc.rfc.refs.iter().any(|ref_id| ref_id == target_id) {
            referenced_by.push(rfc.rfc.rfc_id.clone());
        }
    }

    for adr in &index.adrs {
        if adr
            .spec
            .govctl
            .refs
            .iter()
            .any(|ref_id| ref_id == target_id)
        {
            referenced_by.push(adr.spec.govctl.id.clone());
        }
    }

    for work in &index.work_items {
        if skip_work_id == Some(work.spec.govctl.id.as_str()) {
            continue;
        }

        let has_ref = work
            .spec
            .govctl
            .refs
            .iter()
            .any(|ref_id| ref_id == target_id);
        let has_dependency = include_work_dependencies
            && work
                .spec
                .govctl
                .depends_on
                .iter()
                .any(|dep_id| dep_id == target_id);

        if has_ref || has_dependency {
            referenced_by.push(work.spec.govctl.id.clone());
        }
    }

    referenced_by
}

fn guard_referrers(config: &Config, target_id: &str) -> DiagnosticResult<Vec<String>> {
    Ok(crate::parse::load_guards(config)?
        .into_iter()
        .filter(|guard| guard.meta().refs.iter().any(|ref_id| ref_id == target_id))
        .map(|guard| guard.meta().id.clone())
        .collect())
}
