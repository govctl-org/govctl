use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::write::{WriteOp, read_clause, write_clause};
use std::path::{Path, PathBuf};

pub(super) fn rfc_update_paths(config: &Config, rfc_path: &Path) -> DiagnosticResult<Vec<PathBuf>> {
    let mut paths = vec![rfc_path.to_path_buf()];
    for path in clause_toml_paths(rfc_path)? {
        if read_clause(config, &path)?.since.is_none() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn clause_toml_paths(rfc_path: &Path) -> DiagnosticResult<Vec<PathBuf>> {
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
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&clauses_dir).map_err(|err| {
        Diagnostic::io_error(
            "read clauses directory",
            err,
            clauses_dir.display().to_string(),
        )
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::io_error(
                "read clauses directory entry",
                err,
                clauses_dir.display().to_string(),
            )
        })?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Update pending clauses (since: null) with the given version.
///
/// Clauses are created with `since: None` and filled in when the RFC
/// is bumped or finalized.
pub(super) fn fill_pending_clause_versions(
    config: &Config,
    rfc_path: &Path,
    version: &str,
    op: WriteOp,
) -> DiagnosticResult<Vec<String>> {
    let mut pending_clauses = Vec::new();
    for path in clause_toml_paths(rfc_path)? {
        let clause = read_clause(config, &path)?;
        if clause.since.is_none() {
            pending_clauses.push((path, clause));
        }
    }

    // Sort by clause_id for deterministic output order.
    pending_clauses.sort_by_key(|(_, c)| c.clause_id.clone());

    let mut updated_clause_ids = Vec::with_capacity(pending_clauses.len());
    for (path, mut clause) in pending_clauses {
        clause.since = Some(version.to_string());
        write_clause(&path, &clause, op, Some(&config.display_path(&path)))?;
        updated_clause_ids.push(clause.clause_id);
    }

    Ok(updated_clause_ids)
}
