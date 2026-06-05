use crate::cmd::check::{CheckSummary, collect_diagnostics};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Diagnostics};
use crate::loop_state::{LoopState, load_loop_state, loop_state_root, validate_loop_id};
use crate::model::{ClauseEntry, GuardEntry, ProjectIndex, Release};
use crate::parse::{load_guards_with_warnings, load_releases};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct TuiClauseEntry {
    pub rfc_id: String,
    pub clause: ClauseEntry,
}

#[derive(Debug, Clone)]
pub struct TuiLoopEntry {
    pub id: String,
    pub state: Option<LoopState>,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct TuiTagSummary {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TuiSupplement {
    pub guards: Vec<GuardEntry>,
    pub releases: Vec<Release>,
    pub clauses: Vec<TuiClauseEntry>,
    pub loops: Vec<TuiLoopEntry>,
    pub tags: Vec<TuiTagSummary>,
    pub diagnostics: Diagnostics,
    pub check_summary: CheckSummary,
}

pub fn load_supplement(config: &Config, index: &ProjectIndex) -> TuiSupplement {
    let mut supplement = TuiSupplement {
        clauses: index
            .iter_clauses()
            .map(|(rfc, clause)| TuiClauseEntry {
                rfc_id: rfc.rfc.rfc_id.clone(),
                clause: clause.clone(),
            })
            .collect(),
        tags: tag_summaries(config, index),
        ..TuiSupplement::default()
    };

    if let Ok(result) = load_guards_with_warnings(config) {
        supplement.guards = result.items;
    }

    if let Ok(releases) = load_releases(config) {
        supplement.releases = releases.releases;
    }

    match collect_diagnostics(config) {
        Ok((diagnostics, summary)) => {
            supplement.diagnostics.extend(diagnostics);
            supplement.check_summary = summary;
        }
        Err(diagnostic) => supplement.diagnostics.push(diagnostic),
    }

    supplement.loops = load_loop_entries(config);
    supplement
}

fn tag_summaries(config: &Config, index: &ProjectIndex) -> Vec<TuiTagSummary> {
    let mut counts = BTreeMap::<String, usize>::new();
    for tag in &config.tags.allowed {
        counts.entry(tag.clone()).or_default();
    }
    for rfc in &index.rfcs {
        for tag in &rfc.rfc.tags {
            *counts.entry(tag.clone()).or_default() += 1;
        }
        for clause in &rfc.clauses {
            for tag in &clause.spec.tags {
                *counts.entry(tag.clone()).or_default() += 1;
            }
        }
    }
    for adr in &index.adrs {
        for tag in &adr.meta().tags {
            *counts.entry(tag.clone()).or_default() += 1;
        }
    }
    for item in &index.work_items {
        for tag in &item.meta().tags {
            *counts.entry(tag.clone()).or_default() += 1;
        }
    }

    counts
        .into_iter()
        .map(|(name, count)| TuiTagSummary { name, count })
        .collect()
}

fn load_loop_entries(config: &Config) -> Vec<TuiLoopEntry> {
    let root = loop_state_root(config);
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            return vec![TuiLoopEntry {
                id: root.display().to_string(),
                state: None,
                diagnostic: Some(Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    format!("Failed to read loop state directory: {err}"),
                    root.display().to_string(),
                )),
            }];
        }
    };
    let mut loop_ids = Vec::new();
    let mut diagnostics = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            diagnostics.push(TuiLoopEntry {
                id: root.display().to_string(),
                state: None,
                diagnostic: Some(Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    "Failed to read loop state directory entry",
                    root.display().to_string(),
                )),
            });
            continue;
        };
        let Some(loop_id) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        // Implements [[RFC-0007:C-LOOP-VIEWS]]: invalid loop IDs are not authoritative.
        if validate_loop_id(&loop_id).is_ok() {
            loop_ids.push(loop_id);
        }
    }
    loop_ids.sort();

    let mut loops = diagnostics;
    loops.extend(
        loop_ids
            .into_iter()
            // Implements [[RFC-0007:C-LOOP-VIEWS]]: load valid loop states read-only.
            .map(|loop_id| match load_loop_state(config, &loop_id) {
                Ok(state) => TuiLoopEntry {
                    id: loop_id,
                    state: Some(state),
                    diagnostic: None,
                },
                // Implements [[RFC-0007:C-LOOP-VIEWS]]: invalid loop state remains visible.
                Err(diagnostic) => TuiLoopEntry {
                    id: loop_id,
                    state: None,
                    diagnostic: Some(diagnostic),
                },
            }),
    );
    loops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PathsConfig;
    use crate::loop_state::{LoopState, write_loop_state_with_op};
    use crate::write::WriteOp;
    use std::collections::BTreeMap;

    #[test]
    fn load_loop_entries_loads_sorted_persisted_loop_states()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::TempDir::new()?;
        let config = Config {
            gov_root: temp_dir.path().join("gov"),
            paths: PathsConfig {
                docs_output: temp_dir.path().join("docs"),
                agent_dir: temp_dir.path().join(".claude"),
            },
            ..Default::default()
        };

        std::fs::create_dir_all(temp_dir.path().join(".govctl/loops/not-a-loop"))?;
        write_loop_state_with_op(
            &config,
            &loop_state("LOOP-2026-06-06-002", "WI-2026-06-06-002")?,
            WriteOp::Execute,
        )?;
        write_loop_state_with_op(
            &config,
            &loop_state("LOOP-2026-06-06-001", "WI-2026-06-06-001")?,
            WriteOp::Execute,
        )?;

        let loops = load_loop_entries(&config);

        assert_eq!(
            loops
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["LOOP-2026-06-06-001", "LOOP-2026-06-06-002"]
        );
        assert!(loops.iter().all(|entry| entry.state.is_some()));
        Ok(())
    }

    #[test]
    fn load_loop_entries_reports_directory_read_errors() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::TempDir::new()?;
        let config = Config {
            gov_root: temp_dir.path().join("gov"),
            paths: PathsConfig {
                docs_output: temp_dir.path().join("docs"),
                agent_dir: temp_dir.path().join(".claude"),
            },
            ..Default::default()
        };
        std::fs::create_dir_all(temp_dir.path().join(".govctl"))?;
        std::fs::write(temp_dir.path().join(".govctl/loops"), "not a directory")?;

        let loops = load_loop_entries(&config);

        assert_eq!(loops.len(), 1);
        assert!(loops[0].state.is_none());
        let diagnostic = loops[0]
            .diagnostic
            .as_ref()
            .ok_or_else(|| std::io::Error::other("missing diagnostic"))?;
        assert_eq!(diagnostic.code, DiagnosticCode::E0901IoError);
        assert!(
            diagnostic
                .message
                .contains("Failed to read loop state directory")
        );
        Ok(())
    }

    fn loop_state(loop_id: &str, work_id: &str) -> crate::diagnostic::DiagnosticResult<LoopState> {
        let mut dependencies = BTreeMap::new();
        dependencies.insert(work_id.to_string(), Vec::new());
        LoopState::new(
            loop_id,
            vec![work_id.to_string()],
            vec![work_id.to_string()],
            dependencies,
        )
    }
}
