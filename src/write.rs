//! JSON and frontmatter mutation utilities.
//!
//! Implements [[ADR-0006]] global dry-run support for content-modifying commands.
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChangelogCategory, ChangelogEntry, ClauseSpec, RfcSpec};
use crate::ui;
use anyhow::{Context, Result};
use chrono::Local;
use semver::Version;
use std::path::Path;

/// Parsed changelog change with category and message
#[derive(Debug, Clone)]
pub struct ParsedChange {
    pub category: ChangelogCategory,
    pub message: String,
    /// Whether the category was explicitly specified via prefix
    pub explicit: bool,
}

/// Parse a change string with optional prefix (per ADR-0012).
///
/// Format: `[prefix:] message`
/// - `fix: memory leak` → Fixed category, "memory leak"
/// - `security: patched CVE` → Security category, "patched CVE"
/// - `just a change` → Added category (default), "just a change"
///
/// Returns error if prefix is present but invalid.
pub fn parse_changelog_change(change: &str) -> Result<ParsedChange> {
    // Look for prefix pattern: "word: rest"
    if let Some(colon_pos) = change.find(':') {
        let prefix = change[..colon_pos].trim();
        let message = change[colon_pos + 1..].trim();

        // Only treat as prefix if it's a single word (no spaces)
        if !prefix.contains(' ') && !prefix.is_empty() {
            if let Some(category) = ChangelogCategory::from_prefix(prefix) {
                if message.is_empty() {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0805EmptyValue,
                        format!("Empty message after prefix '{prefix}:'"),
                        "changelog",
                    )
                    .into());
                }
                return Ok(ParsedChange {
                    category,
                    message: message.to_string(),
                    explicit: true,
                });
            } else {
                // Unknown prefix - provide helpful error
                return Err(Diagnostic::new(
                    DiagnosticCode::E0808InvalidPrefix,
                    format!(
                        "Unknown changelog prefix '{prefix}'. Valid prefixes: {}",
                        ChangelogCategory::VALID_PREFIXES.join(", ")
                    ),
                    "changelog",
                )
                .into());
            }
        }
    }

    // No prefix or multi-word before colon - default to Added (not explicit)
    Ok(ParsedChange {
        category: ChangelogCategory::Added,
        message: change.trim().to_string(),
        explicit: false,
    })
}

/// Write operation mode.
///
/// Controls whether write operations execute or just preview.
#[derive(Debug, Clone, Copy, Default)]
pub enum WriteOp {
    /// Actually write to disk
    #[default]
    Execute,
    /// Preview only: show what would be written
    Preview,
}

impl WriteOp {
    /// Create WriteOp from dry_run boolean flag
    pub fn from_dry_run(dry_run: bool) -> Self {
        if dry_run {
            WriteOp::Preview
        } else {
            WriteOp::Execute
        }
    }

    /// Returns true if this is a preview/dry-run operation
    pub fn is_preview(&self) -> bool {
        matches!(self, WriteOp::Preview)
    }
}

/// Write content to a file, respecting WriteOp mode.
///
/// In Preview mode, shows what would be written instead of writing.
/// If `display_path` is provided, it's used for the preview output instead of `path`.
pub fn write_file(path: &Path, content: &str, op: WriteOp, display_path: Option<&Path>) -> Result<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::write(path, content)?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(output_path, content);
        }
    }
    Ok(())
}

/// Create a directory, respecting WriteOp mode.
///
/// In Preview mode, shows what directory would be created.
/// If `display_path` is provided, it's used for the preview output instead of `path`.
pub fn create_dir_all(path: &Path, op: WriteOp, display_path: Option<&Path>) -> Result<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::create_dir_all(path)?;
        }
        WriteOp::Preview => {
            ui::dry_run_mkdir(output_path);
        }
    }
    Ok(())
}

/// Delete a file, respecting WriteOp mode.
///
/// In Preview mode, shows what would be deleted instead of deleting.
/// If `display_path` is provided, it's used for error messages and preview output.
pub fn delete_file(path: &Path, op: WriteOp, display_path: Option<&Path>) -> Result<()> {
    let output_path = display_path.unwrap_or(path);
    match op {
        WriteOp::Execute => {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to delete file: {}", output_path.display()))?;
        }
        WriteOp::Preview => {
            ui::info(format!("[DRY RUN] Would delete: {}", output_path.display()));
        }
    }
    Ok(())
}

/// Version bump level
#[derive(Debug, Clone, Copy)]
pub enum BumpLevel {
    Patch,
    Minor,
    Major,
}

/// Read RFC JSON from file
pub fn read_rfc(path: &Path) -> Result<RfcSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read RFC: {}", path.display()))?;
    let rfc: RfcSpec = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse RFC JSON: {}", path.display()))?;
    Ok(rfc)
}

/// Write RFC JSON to file
pub fn write_rfc(path: &Path, rfc: &RfcSpec, op: WriteOp, display_path: Option<&Path>) -> Result<()> {
    let content = serde_json::to_string_pretty(rfc)?;
    write_file(path, &content, op, display_path)
}

/// Read clause JSON from file
pub fn read_clause(path: &Path) -> Result<ClauseSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read clause: {}", path.display()))?;
    let clause: ClauseSpec = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse clause JSON: {}", path.display()))?;
    Ok(clause)
}

/// Write clause JSON to file
pub fn write_clause(path: &Path, clause: &ClauseSpec, op: WriteOp, display_path: Option<&Path>) -> Result<()> {
    let content = serde_json::to_string_pretty(clause)?;
    write_file(path, &content, op, display_path)
}

/// Bump RFC version and add changelog entry
pub fn bump_rfc_version(rfc: &mut RfcSpec, level: BumpLevel, summary: &str) -> Result<String> {
    let mut version = Version::parse(&rfc.version)
        .with_context(|| format!("Invalid version: {}", rfc.version))?;

    match level {
        BumpLevel::Patch => version.patch += 1,
        BumpLevel::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        BumpLevel::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
    }

    let new_version = version.to_string();
    rfc.version = new_version.clone();
    rfc.updated = Some(today());

    // Add changelog entry (Keep a Changelog format)
    rfc.changelog.insert(
        0,
        ChangelogEntry {
            version: new_version.clone(),
            date: today(),
            notes: Some(summary.to_string()),
            added: vec![],
            changed: vec![],
            deprecated: vec![],
            removed: vec![],
            fixed: vec![],
            security: vec![],
        },
    );

    Ok(new_version)
}

/// Add a change to the current version's changelog.
///
/// Parses prefix from change string per ADR-0012:
/// - `fix: message` → fixed category
/// - `security: message` → security category
/// - `message` (no prefix) → added category
pub fn add_changelog_change(rfc: &mut RfcSpec, change: &str) -> Result<()> {
    let parsed = parse_changelog_change(change)?;

    if let Some(entry) = rfc.changelog.first_mut() {
        match parsed.category {
            ChangelogCategory::Added => entry.added.push(parsed.message),
            ChangelogCategory::Changed => entry.changed.push(parsed.message),
            ChangelogCategory::Deprecated => entry.deprecated.push(parsed.message),
            ChangelogCategory::Removed => entry.removed.push(parsed.message),
            ChangelogCategory::Fixed => entry.fixed.push(parsed.message),
            ChangelogCategory::Security => entry.security.push(parsed.message),
            ChangelogCategory::Chore => {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0809ChoreNotAllowed,
                    "'chore:' category is not valid for RFC changelogs (use for work items only)",
                    "changelog",
                )
                .into());
            }
        }
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0111RfcNoChangelog,
            "No changelog entry exists. Bump version first.",
            "rfc",
        )
        .into());
    }
    Ok(())
}

/// Get today's date in ISO format
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Update a field in RFC JSON
pub fn update_rfc_field(rfc: &mut RfcSpec, field: &str, value: &str) -> Result<(), Diagnostic> {
    match field {
        "status" => {
            rfc.status = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0101RfcSchemaInvalid,
                    format!("Invalid status value: {value}"),
                    "",
                )
            })?;
        }
        "phase" => {
            rfc.phase = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0101RfcSchemaInvalid,
                    format!("Invalid phase value: {value}"),
                    "",
                )
            })?;
        }
        "title" => rfc.title = value.to_string(),
        "version" => rfc.version = value.to_string(),
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0101RfcSchemaInvalid,
                format!("Unknown field: {field}"),
                "",
            ));
        }
    }
    rfc.updated = Some(today());
    Ok(())
}

/// Update a field in clause JSON
pub fn update_clause_field(
    clause: &mut ClauseSpec,
    field: &str,
    value: &str,
) -> Result<(), Diagnostic> {
    match field {
        "text" => clause.text = value.to_string(),
        "title" => clause.title = value.to_string(),
        "status" => {
            clause.status = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!("Invalid status value: {value}"),
                    "",
                )
            })?;
        }
        "kind" => {
            clause.kind = serde_json::from_str(&format!("\"{value}\"")).map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!("Invalid kind value: {value}"),
                    "",
                )
            })?;
        }
        "superseded_by" => {
            clause.superseded_by = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
        "since" => clause.since = Some(value.to_string()),
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0201ClauseSchemaInvalid,
                format!("Unknown field: {field}"),
                "",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_changelog_no_prefix() {
        let result = parse_changelog_change("Added new feature").unwrap();
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "Added new feature");
        assert!(!result.explicit, "no prefix means not explicit");
    }

    #[test]
    fn test_parse_changelog_fix_prefix() {
        let result = parse_changelog_change("fix: memory leak in parser").unwrap();
        assert_eq!(result.category, ChangelogCategory::Fixed);
        assert_eq!(result.message, "memory leak in parser");
        assert!(result.explicit, "prefix means explicit");
    }

    #[test]
    fn test_parse_changelog_security_prefix() {
        let result = parse_changelog_change("security: patched CVE-2026-1234").unwrap();
        assert_eq!(result.category, ChangelogCategory::Security);
        assert_eq!(result.message, "patched CVE-2026-1234");
    }

    #[test]
    fn test_parse_changelog_changed_prefix() {
        let result = parse_changelog_change("changed: API response format").unwrap();
        assert_eq!(result.category, ChangelogCategory::Changed);
        assert_eq!(result.message, "API response format");
    }

    #[test]
    fn test_parse_changelog_deprecated_prefix() {
        let result = parse_changelog_change("deprecated: old API endpoint").unwrap();
        assert_eq!(result.category, ChangelogCategory::Deprecated);
        assert_eq!(result.message, "old API endpoint");
    }

    #[test]
    fn test_parse_changelog_removed_prefix() {
        let result = parse_changelog_change("removed: legacy feature").unwrap();
        assert_eq!(result.category, ChangelogCategory::Removed);
        assert_eq!(result.message, "legacy feature");
    }

    #[test]
    fn test_parse_changelog_add_prefix() {
        let result = parse_changelog_change("add: new CLI flag").unwrap();
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "new CLI flag");
    }

    #[test]
    fn test_parse_changelog_case_insensitive() {
        let result = parse_changelog_change("FIX: uppercase prefix").unwrap();
        assert_eq!(result.category, ChangelogCategory::Fixed);
        assert_eq!(result.message, "uppercase prefix");
    }

    #[test]
    fn test_parse_changelog_invalid_prefix() {
        let result = parse_changelog_change("invalid: some message");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown changelog prefix"));
        assert!(err.contains("Valid prefixes"));
    }

    #[test]
    fn test_parse_changelog_empty_message_after_prefix() {
        let result = parse_changelog_change("fix:");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Empty message after prefix"));
    }

    #[test]
    fn test_parse_changelog_colon_in_message_no_prefix() {
        // "Multi word prefix: message" should not be treated as a prefix
        let result = parse_changelog_change("Updated module: fixed edge case").unwrap();
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "Updated module: fixed edge case");
        assert!(
            !result.explicit,
            "multi-word before colon means not explicit"
        );
    }

    #[test]
    fn test_parse_changelog_url_in_message() {
        // URLs contain colons but shouldn't trigger prefix parsing
        let result = parse_changelog_change("See https://example.com for details").unwrap();
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "See https://example.com for details");
        assert!(!result.explicit, "URL colon means not explicit");
    }
}
