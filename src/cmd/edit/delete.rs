use super::adapter::{DocAdapter, RfcTomlAdapter, TomlAdapter, WorkTomlAdapter};
use crate::cmd::confirmation::confirm_destructive_action;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use crate::write::{WriteOp, delete_file};
use std::collections::BTreeSet;
use std::path::Path;

pub fn delete_clause(
    config: &Config,
    clause_id: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Vec<Diagnostic>> {
    use crate::model::RfcStatus;

    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            "Invalid clause ID format. Expected RFC-NNNN:C-NAME",
            clause_id,
        ));
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfc_loaded = RfcTomlAdapter::load(config, rfc_id)?;
    if rfc_loaded.data.status != RfcStatus::Draft {
        return Err(Diagnostic::new(
            DiagnosticCode::E0110RfcInvalidId,
            format!(
                "Cannot delete clause: {} is {}. Only draft RFCs allow clause deletion.",
                rfc_id,
                rfc_loaded.data.status.as_ref()
            ),
            clause_id,
        ));
    }

    let clause_path = crate::load::find_clause_toml(config, clause_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Clause not found: {}", clause_id),
            clause_id,
        )
    })?;

    let clause_file_name = clause_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0204ClausePathInvalid,
                format!("Invalid clause file path: {}", clause_path.display()),
                clause_id,
            )
        })?;

    ensure_clause_not_referenced(config, clause_id)?;

    if !confirm_destructive_action(
        force,
        op,
        &format!("Delete clause {} from {}?", clause_name, rfc_id),
        "Deletion cancelled",
    )? {
        return Ok(vec![]);
    }

    let mut rfc = rfc_loaded.data.clone();
    let clause_rel_path = format!("clauses/{}", clause_file_name);

    let mut removed = false;
    for section in &mut rfc.sections {
        if let Some(pos) = section.clauses.iter().position(|c| c == &clause_rel_path) {
            section.clauses.remove(pos);
            removed = true;
            break;
        }
    }

    if !removed {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!(
                "Clause {} not found in any section of {}",
                clause_name, rfc_id
            ),
            clause_id,
        ));
    }

    crate::write::write_rfc(
        &rfc_loaded.path,
        &rfc,
        op,
        Some(&config.display_path(&rfc_loaded.path)),
    )?;

    delete_file(&clause_path, op, Some(&config.display_path(&clause_path)))?;

    if !op.is_preview() {
        ui::success(format!("Deleted clause {}", clause_id));
    }

    Ok(vec![])
}

fn ensure_clause_not_referenced(config: &Config, clause_id: &str) -> DiagnosticResult<()> {
    let load_result = crate::load::load_project_with_warnings(config).map_err(|diagnostics| {
        diagnostics.into_iter().next().unwrap_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                "Failed to load project before clause deletion",
                clause_id,
            )
        })
    })?;
    let mut referenced_by = BTreeSet::new();

    for rfc in &load_result.index.rfcs {
        if rfc.rfc.refs.iter().any(|ref_id| ref_id == clause_id) {
            referenced_by.insert(rfc.rfc.rfc_id.clone());
        }
    }

    for adr in &load_result.index.adrs {
        if adr.meta().refs.iter().any(|ref_id| ref_id == clause_id) {
            referenced_by.insert(adr.meta().id.clone());
        }
    }

    for work in &load_result.index.work_items {
        if work.meta().refs.iter().any(|ref_id| ref_id == clause_id) {
            referenced_by.insert(work.meta().id.clone());
        }
    }

    for guard in crate::parse::load_guards(config)? {
        if guard.meta().refs.iter().any(|ref_id| ref_id == clause_id) {
            referenced_by.insert(guard.meta().id.clone());
        }
    }

    if !referenced_by.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0211ClauseStillReferenced,
            format!(
                "Cannot delete clause: {clause_id} is referenced by: {}. Remove references first.",
                referenced_by.into_iter().collect::<Vec<_>>().join(", ")
            ),
            clause_id,
        ));
    }

    Ok(())
}

pub fn delete_work_item(
    config: &Config,
    id: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Vec<Diagnostic>> {
    use crate::load::load_project_with_warnings;
    use crate::model::WorkItemStatus;

    let entry = WorkTomlAdapter::load(config, id)?;
    let wi = &entry.spec;

    if wi.govctl.status != WorkItemStatus::Queue {
        return Err(Diagnostic::new(
            DiagnosticCode::E0402WorkNotFound,
            format!(
                "Cannot delete work item: {} is {}. Only queued work items can be deleted. Use 'mv {} cancelled' for active items.",
                wi.govctl.id,
                wi.govctl.status.as_ref(),
                wi.govctl.id
            ),
            id,
        ));
    }

    let load_result = match load_project_with_warnings(config) {
        Ok(result) => result,
        Err(_) => {
            return proceed_with_deletion(config, &entry.path, &wi.govctl.id, force, op);
        }
    };

    let index = &load_result.index;
    let mut referenced_by = Vec::new();

    for rfc in &index.rfcs {
        if rfc.rfc.refs.contains(&wi.govctl.id) {
            referenced_by.push(rfc.rfc.rfc_id.clone());
        }
    }

    for adr in &index.adrs {
        if adr.spec.govctl.refs.contains(&wi.govctl.id) {
            referenced_by.push(adr.spec.govctl.id.clone());
        }
    }

    for other_wi in &index.work_items {
        if other_wi.spec.govctl.id != wi.govctl.id
            && (other_wi.spec.govctl.refs.contains(&wi.govctl.id)
                || other_wi.spec.govctl.depends_on.contains(&wi.govctl.id))
        {
            referenced_by.push(other_wi.spec.govctl.id.clone());
        }
    }

    if !referenced_by.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0404WorkRefNotFound,
            format!(
                "Cannot delete work item: {} is referenced by: {}. Remove references first.",
                wi.govctl.id,
                referenced_by.join(", ")
            ),
            id,
        ));
    }

    proceed_with_deletion(config, &entry.path, &wi.govctl.id, force, op)
}

fn proceed_with_deletion(
    config: &Config,
    path: &Path,
    id: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Vec<Diagnostic>> {
    if !confirm_destructive_action(
        force,
        op,
        &format!("Delete work item {}?", id),
        "Deletion cancelled",
    )? {
        return Ok(vec![]);
    }

    delete_file(path, op, Some(&config.display_path(path)))?;

    if !op.is_preview() {
        ui::success(format!("Deleted work item {}", id));
    }

    Ok(vec![])
}
