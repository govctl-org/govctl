use super::adapter::{ClauseTomlAdapter, DocAdapter, RfcTomlAdapter, TomlAdapter, WorkTomlAdapter};
use super::delete_referrers::{clause_deletion_referrers, work_item_deletion_referrers};
use crate::cmd::confirmation::confirm_destructive_action;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::load::split_clause_id;
use crate::model::{RfcPhase, RfcStatus};
use crate::ui;
use crate::write::{WriteOp, delete_file, with_file_transaction};
use std::path::Path;

pub fn delete_clause(
    config: &Config,
    clause_id: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Vec<Diagnostic>> {
    let (rfc_id, clause_name) = split_clause_id(clause_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            "Invalid clause ID format. Expected RFC-NNNN:C-NAME",
            clause_id,
        )
    })?;

    let mut rfc_loaded = RfcTomlAdapter::load(config, rfc_id)?;
    let clause_loaded = ClauseTomlAdapter::load(config, clause_id)?;
    // [[RFC-0000:C-CLAUSE-DEF]] limits normative deletion to the open candidate.
    let is_current_candidate_clause = rfc_loaded.data.status == RfcStatus::Normative
        && rfc_loaded.data.phase == RfcPhase::Spec
        && clause_loaded.data.since.as_deref() == Some(rfc_loaded.data.version.as_str());
    if rfc_loaded.data.status != RfcStatus::Draft && !is_current_candidate_clause {
        let since = clause_loaded.data.since.as_deref().unwrap_or("pending");
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot delete clause from {} while status={}, phase={}, version={}, and clause since={}. Clause deletion is limited to draft RFCs or Clauses introduced in the current normative spec candidate.",
                rfc_id,
                rfc_loaded.data.status.as_ref(),
                rfc_loaded.data.phase.as_ref(),
                rfc_loaded.data.version,
                since,
            ),
            clause_id,
        ));
    }

    let clause_path = clause_loaded.path;

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

    let clause_rel_path = format!("clauses/{}", clause_file_name);

    if !unlink_clause_from_sections(&mut rfc_loaded.data, &clause_rel_path) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!(
                "Clause {} not found in any section of {}",
                clause_name, rfc_id
            ),
            clause_id,
        ));
    }

    with_file_transaction(
        &[rfc_loaded.path.as_path(), clause_path.as_path()],
        op,
        || {
            crate::write::write_rfc(
                &rfc_loaded.path,
                &rfc_loaded.data,
                op,
                Some(&config.display_path(&rfc_loaded.path)),
            )?;
            delete_file(&clause_path, op, Some(&config.display_path(&clause_path)))
        },
    )?;

    if !op.is_preview() {
        ui::success(format!("Deleted clause {}", clause_id));
    }

    Ok(vec![])
}

fn unlink_clause_from_sections(rfc: &mut crate::model::RfcSpec, clause_rel_path: &str) -> bool {
    for section in &mut rfc.sections {
        if let Some(pos) = section
            .clauses
            .iter()
            .position(|clause| clause == clause_rel_path)
        {
            section.clauses.remove(pos);
            return true;
        }
    }

    false
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
    let referenced_by = clause_deletion_referrers(config, &load_result.index, clause_id)?;

    if !referenced_by.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0211ClauseStillReferenced,
            format!(
                "Cannot delete clause: {clause_id} is referenced by: {}. Remove references first.",
                referenced_by.join(", ")
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
    let referenced_by = work_item_deletion_referrers(index, &wi.govctl.id);

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
