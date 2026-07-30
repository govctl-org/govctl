//! Source code reference scanning.
//!
//! Implements [[RFC-0009]] source selection and ignore semantics.
//!
//! Scans included, non-ignored files for references to governance artifacts
//! and validates they exist in the project index.

use crate::artifact_index::{ArtifactRefState, artifact_ref_states};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;
use crate::reference_pattern::{self, TargetCaptureError};
use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

/// Result of source scanning
#[derive(Debug, Default)]
pub struct ScanResult {
    pub diagnostics: Vec<Diagnostic>,
    pub files_scanned: usize,
    pub refs_found: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SourceReference {
    path: String,
    line: usize,
    byte_column: usize,
    target: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct InvalidReferenceMatch {
    path: String,
    line: usize,
    byte_column: usize,
    error: TargetCaptureError,
}

struct SourceReferenceDiagnostic {
    path: String,
    line: usize,
    byte_column: usize,
    code: DiagnosticCode,
    target: String,
    diagnostic: Diagnostic,
}

struct SourceLocator<'content> {
    content: &'content [u8],
    scanned_to: usize,
    line: usize,
    line_start: usize,
}

impl<'content> SourceLocator<'content> {
    fn new(content: &'content str) -> Self {
        Self {
            content: content.as_bytes(),
            scanned_to: 0,
            line: 1,
            line_start: 0,
        }
    }

    fn position(&mut self, byte_offset: usize) -> (usize, usize) {
        debug_assert!(byte_offset >= self.scanned_to);
        for index in self.scanned_to..byte_offset {
            if self.content[index] == b'\n' {
                self.line += 1;
                self.line_start = index + 1;
            }
        }
        self.scanned_to = byte_offset;
        (self.line, byte_offset - self.line_start + 1)
    }
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
    let pattern = match reference_pattern::compile(
        &config.source_scan.pattern,
        config
            .display_path(&config.gov_root.join("config.toml"))
            .display()
            .to_string(),
    ) {
        Ok(re) => re,
        Err(diagnostic) => {
            result.diagnostics.push(diagnostic);
            return result;
        }
    };

    let project_root = config.project_root();
    let include_matcher = match build_include_matcher(project_root, &config.source_scan.include) {
        Ok(matcher) => matcher,
        Err(diagnostic) => {
            result.diagnostics.push(diagnostic);
            return result;
        }
    };

    let mut builder = WalkBuilder::new(project_root);
    builder
        .standard_filters(false)
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .add_custom_ignore_filename(".govignore")
        .filter_entry(|entry| entry.file_name() != OsStr::new(".git"));

    let mut references = Vec::new();
    let mut invalid_matches = Vec::new();
    for entry in builder.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    format!("Source scan traversal failed: {error}"),
                    config.display_path(project_root).display().to_string(),
                ));
                continue;
            }
        };
        if let Some(error) = entry.error() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("Invalid source scan ignore rule: {error}"),
                config.display_path(entry.path()).display().to_string(),
            ));
        }

        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            validate_reached_ignore_files(config, entry.path(), &mut result);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let match_path = path.strip_prefix(project_root).unwrap_or(path);

        if !include_matcher
            .matched_path_or_any_parents(path, false)
            .is_ignore()
        {
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                result.diagnostics.push(Diagnostic::io_error(
                    "read selected source file",
                    error,
                    config.display_path(path).display().to_string(),
                ));
                continue;
            }
        };

        result.files_scanned += 1;
        let path_str = normalized_relative_path(match_path);
        let mut locator = SourceLocator::new(&content);

        // Find all matches
        for caps in pattern.captures_iter(&content) {
            match reference_pattern::target_capture(&caps) {
                Ok(target) => {
                    let (line, byte_column) = locator.position(target.start());
                    references.push(SourceReference {
                        path: path_str.clone(),
                        line,
                        byte_column,
                        target: target.as_str().to_string(),
                    });
                }
                Err(error) => {
                    let offset = caps
                        .get(1)
                        .or_else(|| caps.get(0))
                        .map_or(0, |matched| matched.start());
                    let (line, byte_column) = locator.position(offset);
                    invalid_matches.push(InvalidReferenceMatch {
                        path: path_str.clone(),
                        line,
                        byte_column,
                        error,
                    });
                }
            }
        }
    }

    append_invalid_match_diagnostics(&mut invalid_matches, &mut result.diagnostics);
    append_reference_diagnostics(&mut references, &known_ids, &mut result);
    result
}

fn append_invalid_match_diagnostics(
    invalid_matches: &mut Vec<InvalidReferenceMatch>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    invalid_matches.sort();
    invalid_matches.dedup();
    diagnostics.extend(invalid_matches.iter().map(|invalid| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!("Invalid source_scan.pattern match: {}", invalid.error),
            source_location(&invalid.path, invalid.line, invalid.byte_column),
        )
    }));
}

fn append_reference_diagnostics(
    references: &mut Vec<SourceReference>,
    known_ids: &HashMap<String, ArtifactRefState>,
    result: &mut ScanResult,
) {
    references.sort();
    references.dedup();
    result.refs_found = references.len();

    let mut diagnostics = Vec::new();
    for reference in references {
        let location = source_location(&reference.path, reference.line, reference.byte_column);
        match known_ids.get(&reference.target).copied() {
            None => diagnostics.push(SourceReferenceDiagnostic {
                path: reference.path.clone(),
                line: reference.line,
                byte_column: reference.byte_column,
                code: DiagnosticCode::E0107SourceRefUnknown,
                target: reference.target.clone(),
                diagnostic: Diagnostic::new(
                    DiagnosticCode::E0107SourceRefUnknown,
                    format!("Unknown artifact reference: {}", reference.target),
                    location,
                ),
            }),
            Some(ArtifactRefState::Outdated(reason)) => {
                diagnostics.push(SourceReferenceDiagnostic {
                    path: reference.path.clone(),
                    line: reference.line,
                    byte_column: reference.byte_column,
                    code: DiagnosticCode::W0107SourceRefOutdated,
                    target: reference.target.clone(),
                    diagnostic: Diagnostic::new(
                        DiagnosticCode::W0107SourceRefOutdated,
                        format!(
                            "Outdated reference: {} ({}) (hint: update comment or remove [[...]])",
                            reference.target, reason
                        ),
                        location,
                    ),
                });
            }
            Some(ArtifactRefState::Active) => {}
        }
    }
    diagnostics.sort_by(|left, right| {
        (
            &left.path,
            left.line,
            left.byte_column,
            left.code.code(),
            &left.target,
        )
            .cmp(&(
                &right.path,
                right.line,
                right.byte_column,
                right.code.code(),
                &right.target,
            ))
    });
    result
        .diagnostics
        .extend(diagnostics.into_iter().map(|entry| entry.diagnostic));
}

fn build_include_matcher(root: &Path, patterns: &[String]) -> Result<Gitignore, Diagnostic> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        if pattern.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                "Invalid source_scan.include pattern: entries cannot be empty",
                "gov/config.toml".to_string(),
            ));
        }
        if pattern.starts_with('!') {
            return Err(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!(
                    "Invalid source_scan.include pattern '{pattern}': leading '!' is not allowed"
                ),
                "gov/config.toml".to_string(),
            ));
        }

        let gitignore_pattern = if pattern.starts_with('#') {
            format!(r"\{pattern}")
        } else {
            pattern.clone()
        };
        builder
            .add_line(None, &gitignore_pattern)
            .map_err(|error| {
                Diagnostic::new(
                    DiagnosticCode::E0501ConfigInvalid,
                    format!("Invalid source_scan.include pattern '{pattern}': {error}"),
                    "gov/config.toml".to_string(),
                )
            })?;
    }

    builder.build().map_err(|error| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!("Failed to build source_scan.include matcher: {error}"),
            "gov/config.toml".to_string(),
        )
    })
}

fn normalized_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn source_location(path: &str, line: usize, byte_column: usize) -> String {
    format!("{path}:{line}:{byte_column}")
}

fn validate_reached_ignore_files(config: &Config, directory: &Path, result: &mut ScanResult) {
    for name in [".gitignore", ".govignore"] {
        let path = directory.join(name);
        match fs::read_to_string(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => result.diagnostics.push(Diagnostic::io_error(
                "read source scan ignore file",
                error,
                config.display_path(&path).display().to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_diagnostics_deduplicate_identical_occurrences() {
        let duplicate = SourceReference {
            path: "src/main.rs".to_string(),
            line: 2,
            byte_column: 4,
            target: "RFC-9999".to_string(),
        };
        let mut references = vec![duplicate.clone(), duplicate];
        let mut result = ScanResult::default();

        append_reference_diagnostics(&mut references, &HashMap::new(), &mut result);

        assert_eq!(result.refs_found, 1);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].code,
            DiagnosticCode::E0107SourceRefUnknown
        );
        assert_eq!(result.diagnostics[0].file, "src/main.rs:2:4");
    }
}
