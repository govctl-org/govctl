use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use crate::write::{WriteOp, read_clause, write_clause};
use std::path::Path;

/// Update pending clauses (since: null) with the given version.
///
/// Clauses are created with `since: None` and filled in when the RFC
/// is bumped or finalized.
pub(super) fn fill_pending_clause_versions(
    config: &Config,
    rfc_path: &Path,
    version: &str,
    op: WriteOp,
) -> DiagnosticResult<()> {
    let clauses_dir = rfc_path
        .parent()
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0901IoError,
                "RFC path has no parent directory",
                rfc_path.display().to_string(),
            )
        })?
        .join("clauses");
    if !clauses_dir.exists() {
        return Ok(());
    }

    let mut pending_clauses: Vec<_> = std::fs::read_dir(&clauses_dir)
        .map_err(|err| {
            Diagnostic::io_error(
                "read clauses directory",
                err,
                clauses_dir.display().to_string(),
            )
        })?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .filter_map(|p| read_clause(config, &p).ok().map(|c| (p, c)))
        .filter(|(_, c)| c.since.is_none())
        .collect();

    // Sort by clause_id for deterministic output order.
    pending_clauses.sort_by_key(|(_, c)| c.clause_id.clone());

    for (path, mut clause) in pending_clauses {
        clause.since = Some(version.to_string());
        write_clause(&path, &clause, op, Some(&config.display_path(&path)))?;
        if !op.is_preview() {
            ui::sub_info(format!("Set {}.since = {}", clause.clause_id, version));
        }
    }

    Ok(())
}
