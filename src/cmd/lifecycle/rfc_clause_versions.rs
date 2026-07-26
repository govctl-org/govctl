use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult};
use crate::write::{WriteOp, read_clause, write_clause};
use std::path::{Path, PathBuf};

pub(super) fn rfc_update_paths(config: &Config, rfc_path: &Path) -> DiagnosticResult<Vec<PathBuf>> {
    let mut paths = vec![rfc_path.to_path_buf()];
    for path in clause_toml_paths(config, rfc_path)? {
        if read_clause(config, &path)?.since.is_none() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

pub(super) fn pending_clause_ids(
    config: &Config,
    rfc_path: &Path,
) -> DiagnosticResult<Vec<String>> {
    let mut clause_ids = Vec::new();
    for path in clause_toml_paths(config, rfc_path)? {
        let clause = read_clause(config, &path)?;
        if clause.since.is_none() {
            clause_ids.push(clause.clause_id);
        }
    }
    clause_ids.sort();
    Ok(clause_ids)
}

fn clause_toml_paths(config: &Config, rfc_path: &Path) -> DiagnosticResult<Vec<PathBuf>> {
    let mut paths = crate::load::load_rfc(config, rfc_path)
        .map_err(Diagnostic::from)?
        .clauses
        .into_iter()
        .map(|clause| clause.path)
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

/// Update pending clauses (since: null) with the given version.
///
/// Pending clauses are filled in when the RFC is bumped or finalized.
pub(super) fn fill_pending_clause_versions(
    config: &Config,
    rfc_path: &Path,
    version: &str,
    op: WriteOp,
) -> DiagnosticResult<Vec<String>> {
    let mut pending_clauses = Vec::new();
    for path in clause_toml_paths(config, rfc_path)? {
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
