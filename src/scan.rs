//! Source code reference scanning.
//!
//! Implements [[ADR-0009]] configurable source code reference scanning.
//!
//! Scans files matching include/exclude glob patterns for references to
//! governance artifacts and validates they exist in the project index.

use crate::artifact_index::{ArtifactRefState, artifact_ref_states};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;
use globset::{Glob, GlobSet, GlobSetBuilder};
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
    let known_ids = artifact_ref_states(index);

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

    let include_set = match build_glob_set(&config.source_scan.include, "include") {
        Ok(set) => set,
        Err(diagnostic) => {
            result.diagnostics.push(diagnostic);
            return result;
        }
    };
    let exclude_set = match build_glob_set(&config.source_scan.exclude, "exclude") {
        Ok(set) => set,
        Err(diagnostic) => {
            result.diagnostics.push(diagnostic);
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
            match known_ids.get(artifact_id).copied() {
                None => {
                    result.diagnostics.push(Diagnostic::new(
                        DiagnosticCode::E0107SourceRefUnknown,
                        format!("Unknown artifact reference: {}", artifact_id),
                        path_str.clone(),
                    ));
                }
                Some(ArtifactRefState::Outdated(reason)) => {
                    result.diagnostics.push(Diagnostic::new(
                        DiagnosticCode::W0107SourceRefOutdated,
                        format!(
                            "Outdated reference: {} ({}) (hint: update comment or remove [[...]])",
                            artifact_id, reason
                        ),
                        path_str.clone(),
                    ));
                }
                Some(ArtifactRefState::Active) => {
                    // OK - reference is valid
                }
            }
        }
    }

    result
}

fn build_glob_set(patterns: &[String], label: &str) -> Result<GlobSet, Diagnostic> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        let glob = Glob::new(pat).map_err(|e| {
            Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("Invalid source_scan.{label} glob '{}': {}", pat, e),
                "gov/config.toml".to_string(),
            )
        })?;
        builder.add(glob);
    }

    builder.build().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!("Failed to build {label} glob set: {}", e),
            "gov/config.toml".to_string(),
        )
    })
}

// Tests moved to tests/cli_snapshots.rs using fixtures/source_scan
