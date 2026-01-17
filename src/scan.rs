//! Source code reference scanning.
//!
//! Scans configured directories for references to governance artifacts
//! and validates they exist in the project index.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseStatus, ProjectIndex, RfcStatus};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use walkdir::WalkDir;

/// Result of source scanning
#[derive(Debug, Default)]
pub struct ScanResult {
    pub diagnostics: Vec<Diagnostic>,
    pub files_scanned: usize,
    pub refs_found: usize,
}

/// Scan source files for artifact references
pub fn scan_source_refs(config: &Config, index: &ProjectIndex) -> ScanResult {
    if !config.source_scan.enabled {
        return ScanResult::default();
    }

    let mut result = ScanResult::default();

    // Build known artifact IDs
    let known_ids = build_artifact_index(index);

    // Compile the pattern
    let pattern = match Regex::new(&config.source_scan.pattern) {
        Ok(re) => re,
        Err(e) => {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("Invalid source_scan.pattern regex: {}", e),
                "gov/config.toml".to_string(),
            ));
            return result;
        }
    };

    // Collect file extensions for filtering
    let exts: HashSet<&str> = config.source_scan.exts.iter().map(|s| s.as_str()).collect();

    // Walk configured roots
    for root in &config.source_scan.roots {
        if !root.exists() {
            continue;
        }

        let files = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| exts.contains(ext))
            });

        for entry in files {
            let path = entry.path();
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };

            result.files_scanned += 1;
            let path_str = path.to_string_lossy().to_string();

            // Find all matches
            for caps in pattern.captures_iter(&content) {
                let Some(artifact_id) = caps.get(1).map(|m| m.as_str()) else {
                    continue;
                };

                result.refs_found += 1;

                // Check if artifact exists
                match known_ids.get(artifact_id) {
                    None => {
                        result.diagnostics.push(Diagnostic::new(
                            DiagnosticCode::E0107SourceRefUnknown,
                            format!("Unknown artifact reference: {}", artifact_id),
                            path_str.clone(),
                        ));
                    }
                    Some(ArtifactState::Outdated(reason)) => {
                        result.diagnostics.push(Diagnostic::new(
                            DiagnosticCode::W0107SourceRefOutdated,
                            format!("Outdated artifact reference: {} ({})", artifact_id, reason),
                            path_str.clone(),
                        ));
                    }
                    Some(ArtifactState::Active) => {
                        // OK - reference is valid
                    }
                }
            }
        }
    }

    result
}

/// State of an artifact for reference validation
#[derive(Debug, Clone)]
enum ArtifactState {
    Active,
    Outdated(String),
}

/// Build index of known artifact IDs with their state
fn build_artifact_index(index: &ProjectIndex) -> std::collections::HashMap<String, ArtifactState> {
    let mut known = std::collections::HashMap::new();

    // Add RFC IDs
    for rfc in &index.rfcs {
        let state = match rfc.rfc.status {
            RfcStatus::Deprecated => ArtifactState::Outdated("deprecated".to_string()),
            _ => ArtifactState::Active,
        };
        known.insert(rfc.rfc.rfc_id.clone(), state);

        // Add clause IDs
        for clause in &rfc.clauses {
            let clause_id = format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id);
            let state = match clause.spec.status {
                ClauseStatus::Superseded => ArtifactState::Outdated("superseded".to_string()),
                ClauseStatus::Deprecated => ArtifactState::Outdated("deprecated".to_string()),
                ClauseStatus::Active => {
                    // If parent RFC is deprecated, clause is also outdated
                    if rfc.rfc.status == RfcStatus::Deprecated {
                        ArtifactState::Outdated("RFC deprecated".to_string())
                    } else {
                        ArtifactState::Active
                    }
                }
            };
            known.insert(clause_id, state);
        }
    }

    // Add ADR IDs
    for adr in &index.adrs {
        let state = match adr.meta().status {
            crate::model::AdrStatus::Superseded => {
                ArtifactState::Outdated("superseded".to_string())
            }
            _ => ArtifactState::Active,
        };
        known.insert(adr.meta().id.clone(), state);
    }

    // Add Work Item IDs (optional, but for completeness)
    for work in &index.work_items {
        known.insert(work.meta().id.clone(), ArtifactState::Active);
    }

    known
}

// Tests moved to tests/cli_snapshots.rs using fixtures/source_scan
