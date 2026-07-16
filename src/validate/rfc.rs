use super::ValidationResult;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseStatus, ProjectIndex, RfcIndex, RfcPhase, RfcStatus};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClauseSupersessionVisit {
    Visiting,
    Visited,
}

pub(super) fn validate_rfc(rfc: &RfcIndex, config: &Config, result: &mut ValidationResult) {
    let rfc_path_display = config.display_path(&rfc.path).display().to_string();

    let dir_name = rfc
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    if let Some(name) = dir_name
        && name != rfc.rfc.rfc_id
    {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0103RfcIdMismatch,
            format!(
                "RFC ID '{}' doesn't match directory '{}'",
                rfc.rfc.rfc_id, name
            ),
            rfc_path_display.clone(),
        ));
    }

    let current_changelog_count = rfc
        .rfc
        .changelog
        .iter()
        .filter(|entry| entry.version == rfc.rfc.version)
        .count();
    if current_changelog_count != 1 {
        let code = if current_changelog_count == 0 {
            DiagnosticCode::E0111RfcNoChangelog
        } else {
            DiagnosticCode::E0115RfcCurrentChangelogInvalid
        };
        result.diagnostics.push(Diagnostic::new(
            code,
            format!(
                "RFC must contain exactly one changelog entry for current version {} (found {})",
                rfc.rfc.version, current_changelog_count
            ),
            rfc_path_display.clone(),
        ));
    }

    validate_status_phase_constraints(rfc, config, result);

    for clause in &rfc.clauses {
        let clause_path_display = config.display_path(&clause.path).display().to_string();
        if clause.spec.since.is_none() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0102ClauseNoSince,
                format!(
                    "Clause '{}' has no 'since' version (hint: it will be set automatically by `govctl rfc bump` or `govctl rfc finalize`)",
                    clause.spec.clause_id
                ),
                clause_path_display.clone(),
            ));
        }

        let file_name = clause
            .path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name != clause.spec.clause_id {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0203ClauseIdMismatch,
                format!(
                    "Clause ID '{}' doesn't match filename '{}'",
                    clause.spec.clause_id, file_name
                ),
                clause_path_display,
            ));
        }
    }
}

fn validate_status_phase_constraints(
    rfc: &RfcIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let status = rfc.rfc.status;
    let phase = rfc.rfc.phase;
    let path_display = config.display_path(&rfc.path).display().to_string();

    if status == RfcStatus::Draft && phase == RfcPhase::Stable {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            "Cannot have status=draft with phase=stable",
            path_display.clone(),
        ));
    }

    if status == RfcStatus::Deprecated && (phase == RfcPhase::Impl || phase == RfcPhase::Test) {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot have status=deprecated with phase={}",
                phase.as_ref()
            ),
            path_display,
        ));
    }
}

pub(super) fn validate_clause_references(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let known_clauses: HashSet<String> = index
        .iter_clauses()
        .map(|(rfc, c)| format!("{}:{}", rfc.rfc.rfc_id, c.spec.clause_id))
        .collect();
    let mut graph = HashMap::new();
    let mut path_by_id = HashMap::new();

    for (rfc, clause) in index.iter_clauses() {
        if clause.spec.status == ClauseStatus::Superseded && clause.spec.superseded_by.is_none() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0213ClauseSupersededByMissing,
                format!(
                    "Superseded clause '{}' has no superseded_by target",
                    clause.spec.clause_id
                ),
                config.display_path(&clause.path).display().to_string(),
            ));
        }

        if let Some(ref superseded_by) = clause.spec.superseded_by {
            let source = format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id);
            let clause_path_display = config.display_path(&clause.path).display().to_string();
            path_by_id.insert(source.clone(), clause_path_display.clone());
            if clause.spec.status != ClauseStatus::Superseded {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0206ClauseSupersededByUnknown,
                    format!(
                        "Clause has superseded_by but status is not 'superseded': {}",
                        clause.spec.clause_id
                    ),
                    clause_path_display.clone(),
                ));
            }

            let full_ref = if superseded_by.contains(':') {
                superseded_by.clone()
            } else {
                format!("{}:{}", rfc.rfc.rfc_id, superseded_by)
            };

            if !known_clauses.contains(&full_ref) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0206ClauseSupersededByUnknown,
                    format!(
                        "Clause '{}' superseded by unknown clause '{}'",
                        clause.spec.clause_id, superseded_by
                    ),
                    clause_path_display,
                ));
                continue;
            }

            graph.insert(source, full_ref);
        }
    }

    let mut state = HashMap::new();
    let mut stack = Vec::new();
    let mut clause_ids: Vec<_> = graph.keys().cloned().collect();
    clause_ids.sort();

    for clause_id in clause_ids {
        if state.contains_key(&clause_id) {
            continue;
        }

        if let Some(cycle) =
            detect_clause_supersession_cycle(&clause_id, &graph, &mut state, &mut stack)
        {
            let cycle_start = cycle.first().unwrap_or(&clause_id);
            let path = path_by_id
                .get(cycle_start)
                .cloned()
                .unwrap_or_else(|| config.display_path(&config.rfc_dir()).display().to_string());
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0212ClauseSupersessionCycle,
                format!("Clause supersession cycle detected: {}", cycle.join(" -> ")),
                path,
            ));
            break;
        }
    }
}

// Persisted `superseded_by` values are direct historical edges per
// [[RFC-0001:C-CLAUSE-STATUS]].
fn detect_clause_supersession_cycle(
    clause_id: &str,
    graph: &HashMap<String, String>,
    state: &mut HashMap<String, ClauseSupersessionVisit>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    let mut current = clause_id.to_string();
    loop {
        match state.get(&current) {
            Some(ClauseSupersessionVisit::Visiting) => {
                let start = stack.iter().position(|id| id == &current).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(current);
                return Some(cycle);
            }
            Some(ClauseSupersessionVisit::Visited) => break,
            None => {}
        }

        state.insert(current.clone(), ClauseSupersessionVisit::Visiting);
        stack.push(current.clone());

        let Some(target) = graph
            .get(&current)
            .filter(|target| graph.contains_key(*target))
        else {
            break;
        };
        current.clone_from(target);
    }

    for visited in stack.drain(..) {
        state.insert(visited, ClauseSupersessionVisit::Visited);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_acyclic_supersession_chain_does_not_overflow_the_stack() {
        const EDGE_COUNT: usize = 20_000;
        let graph: HashMap<_, _> = (0..EDGE_COUNT)
            .map(|index| {
                (
                    format!("RFC-0001:C-{index}"),
                    format!("RFC-0001:C-{}", index + 1),
                )
            })
            .collect();
        let mut state = HashMap::new();
        let mut stack = Vec::new();

        assert_eq!(
            detect_clause_supersession_cycle("RFC-0001:C-0", &graph, &mut state, &mut stack,),
            None
        );
        assert_eq!(state.len(), graph.len());
    }
}
