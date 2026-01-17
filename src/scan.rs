//! Source code reference scanning.
//!
//! Implements [[ADR-0009]] configurable source code reference scanning.
//!
//! Scans files matching include/exclude glob patterns for references to
//! governance artifacts and validates they exist in the project index.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseStatus, ProjectIndex, RfcStatus};
use globset::{Glob, GlobSetBuilder};
use regex::Regex;
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

    // Compile the artifact pattern
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

    // Build include glob set
    let mut include_builder = GlobSetBuilder::new();
    for pat in &config.source_scan.include {
        match Glob::new(pat) {
            Ok(g) => {
                include_builder.add(g);
            }
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    format!("Invalid source_scan.include glob '{}': {}", pat, e),
                    "gov/config.toml".to_string(),
                ));
                return result;
            }
        }
    }
    let include_set = match include_builder.build() {
        Ok(s) => s,
        Err(e) => {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("Failed to build include glob set: {}", e),
                "gov/config.toml".to_string(),
            ));
            return result;
        }
    };

    // Build exclude glob set
    let mut exclude_builder = GlobSetBuilder::new();
    for pat in &config.source_scan.exclude {
        match Glob::new(pat) {
            Ok(g) => {
                exclude_builder.add(g);
            }
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    format!("Invalid source_scan.exclude glob '{}': {}", pat, e),
                    "gov/config.toml".to_string(),
                ));
                return result;
            }
        }
    }
    let exclude_set = match exclude_builder.build() {
        Ok(s) => s,
        Err(e) => {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("Failed to build exclude glob set: {}", e),
                "gov/config.toml".to_string(),
            ));
            return result;
        }
    };

    // Walk from current directory, filter by include/exclude
    let files = WalkDir::new(".")
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in files {
        let path = entry.path();
        // Strip leading "./" for glob matching
        let match_path = path.strip_prefix("./").unwrap_or(path);

        // Check include/exclude
        if !include_set.is_match(match_path) || exclude_set.is_match(match_path) {
            continue;
        }

        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };

        result.files_scanned += 1;
        let path_str = match_path.to_string_lossy().to_string();

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
