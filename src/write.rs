//! JSON and frontmatter mutation utilities.
//!
//! Implements [[ADR-0006]] global dry-run support for content-modifying commands.
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChangelogCategory, ChangelogEntry, ClauseSpec, ClauseWire, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_json_value, validate_toml_value, with_schema_header};
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
pub fn write_file(
    path: &Path,
    content: &str,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<()> {
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

/// Read RFC from file and validate its normalized structure.
/// Handles both legacy flat format and new `[govctl]` wire format (TOML and JSON).
pub fn read_rfc(config: &Config, path: &Path) -> Result<RfcSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read RFC: {}", path.display()))?;
    let rfc = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            let mut raw: toml::Value = toml::from_str(&content)
                .with_context(|| format!("Failed to parse RFC TOML: {}", path.display()))?;
            normalize_rfc_value(&mut raw);
            validate_toml_value(ArtifactSchema::Rfc, config, path, &raw)?;
            let wire: RfcWire = raw
                .try_into()
                .with_context(|| format!("Failed to deserialize RFC TOML: {}", path.display()))?;
            wire.into()
        }
        _ => {
            let mut raw: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse RFC JSON: {}", path.display()))?;
            normalize_rfc_json(&mut raw);
            validate_json_value(ArtifactSchema::Rfc, config, path, &raw)?;
            let wire: RfcWire = serde_json::from_value(raw)
                .with_context(|| format!("Failed to deserialize RFC JSON: {}", path.display()))?;
            wire.into()
        }
    };
    Ok(rfc)
}

/// Write RFC to file in TOML only.
/// TOML output uses the `[govctl]` wire format plus schema header.
pub fn write_rfc(
    path: &Path,
    rfc: &RfcSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<()> {
    let wire: RfcWire = rfc.clone().into();
    let body = toml::to_string_pretty(&wire)?;
    let content = with_schema_header(ArtifactSchema::Rfc, &body);
    write_file(path, &content, op, display_path)
}

/// Read clause from file and validate its normalized structure.
/// Handles both legacy flat format and new `[govctl]` + `[content]` wire format.
pub fn read_clause(config: &Config, path: &Path) -> Result<ClauseSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read clause: {}", path.display()))?;
    let clause = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            let mut raw: toml::Value = toml::from_str(&content)
                .with_context(|| format!("Failed to parse clause TOML: {}", path.display()))?;
            normalize_clause_value(&mut raw);
            validate_toml_value(ArtifactSchema::Clause, config, path, &raw)?;
            let wire: ClauseWire = raw.try_into().with_context(|| {
                format!("Failed to deserialize clause TOML: {}", path.display())
            })?;
            wire.into()
        }
        _ => {
            let mut raw: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse clause JSON: {}", path.display()))?;
            normalize_clause_json(&mut raw);
            validate_json_value(ArtifactSchema::Clause, config, path, &raw)?;
            let wire: ClauseWire = serde_json::from_value(raw).with_context(|| {
                format!("Failed to deserialize clause JSON: {}", path.display())
            })?;
            wire.into()
        }
    };
    Ok(clause)
}

/// Write clause to file in TOML only.
/// TOML output uses the `[govctl]` + `[content]` wire format plus schema header.
pub fn write_clause(
    path: &Path,
    clause: &ClauseSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<()> {
    let wire: ClauseWire = clause.clone().into();
    let body = toml::to_string_pretty(&wire)?;
    let content = with_schema_header(ArtifactSchema::Clause, &body);
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

/// Normalize a flat RFC TOML value into the `[govctl]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_rfc_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(rfc_id) = root.remove("rfc_id") else {
        return;
    };

    let mut govctl = toml::map::Map::new();
    govctl.insert("schema".to_string(), toml::Value::Integer(1));
    govctl.insert("id".to_string(), rfc_id);
    for key in &[
        "title",
        "version",
        "status",
        "phase",
        "owners",
        "created",
        "updated",
        "supersedes",
        "refs",
        "signature",
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), toml::Value::Table(govctl));
}

/// Normalize a flat clause TOML value into the `[govctl]` + `[content]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_clause_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(clause_id) = root.remove("clause_id") else {
        return;
    };

    let mut govctl = toml::map::Map::new();
    govctl.insert("schema".to_string(), toml::Value::Integer(1));
    govctl.insert("id".to_string(), clause_id);
    for key in &[
        "title",
        "kind",
        "status",
        "anchors",
        "superseded_by",
        "since",
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), toml::Value::Table(govctl));

    let mut content = toml::map::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), toml::Value::Table(content));
}

/// Normalize a flat RFC JSON value into the `govctl` wire layout.
pub(crate) fn normalize_rfc_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(rfc_id) = root.remove("rfc_id") else {
        return;
    };
    let mut govctl = serde_json::Map::new();
    govctl.insert("schema".to_string(), serde_json::json!(1));
    govctl.insert("id".to_string(), rfc_id);
    for key in &[
        "title",
        "version",
        "status",
        "phase",
        "owners",
        "created",
        "updated",
        "supersedes",
        "refs",
        "signature",
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), serde_json::Value::Object(govctl));
}

/// Normalize a flat clause JSON value into the `govctl` + `content` wire layout.
pub(crate) fn normalize_clause_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    if root.contains_key("govctl") {
        return;
    }
    let Some(clause_id) = root.remove("clause_id") else {
        return;
    };
    let mut govctl = serde_json::Map::new();
    govctl.insert("schema".to_string(), serde_json::json!(1));
    govctl.insert("id".to_string(), clause_id);
    for key in &[
        "title",
        "kind",
        "status",
        "anchors",
        "superseded_by",
        "since",
    ] {
        if let Some(v) = root.remove(*key) {
            govctl.insert(key.to_string(), v);
        }
    }
    root.insert("govctl".to_string(), serde_json::Value::Object(govctl));

    let mut content = serde_json::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), serde_json::Value::Object(content));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_changelog_no_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("Added new feature")?;
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "Added new feature");
        assert!(!result.explicit, "no prefix means not explicit");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_fix_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("fix: memory leak in parser")?;
        assert_eq!(result.category, ChangelogCategory::Fixed);
        assert_eq!(result.message, "memory leak in parser");
        assert!(result.explicit, "prefix means explicit");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_security_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("security: patched CVE-2026-1234")?;
        assert_eq!(result.category, ChangelogCategory::Security);
        assert_eq!(result.message, "patched CVE-2026-1234");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_changed_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("changed: API response format")?;
        assert_eq!(result.category, ChangelogCategory::Changed);
        assert_eq!(result.message, "API response format");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_deprecated_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("deprecated: old API endpoint")?;
        assert_eq!(result.category, ChangelogCategory::Deprecated);
        assert_eq!(result.message, "old API endpoint");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_removed_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("removed: legacy feature")?;
        assert_eq!(result.category, ChangelogCategory::Removed);
        assert_eq!(result.message, "legacy feature");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_add_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("add: new CLI flag")?;
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "new CLI flag");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_case_insensitive() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_changelog_change("FIX: uppercase prefix")?;
        assert_eq!(result.category, ChangelogCategory::Fixed);
        assert_eq!(result.message, "uppercase prefix");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_invalid_prefix() {
        let result = parse_changelog_change("invalid: some message");
        assert!(result.is_err());
        let err = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(err.contains("Unknown changelog prefix"));
        assert!(err.contains("Valid prefixes"));
    }

    #[test]
    fn test_parse_changelog_empty_message_after_prefix() {
        let result = parse_changelog_change("fix:");
        assert!(result.is_err());
        let err = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(err.contains("Empty message after prefix"));
    }

    #[test]
    fn test_parse_changelog_colon_in_message_no_prefix() -> Result<(), Box<dyn std::error::Error>> {
        // "Multi word prefix: message" should not be treated as a prefix
        let result = parse_changelog_change("Updated module: fixed edge case")?;
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "Updated module: fixed edge case");
        assert!(
            !result.explicit,
            "multi-word before colon means not explicit"
        );
        Ok(())
    }

    #[test]
    fn test_parse_changelog_url_in_message() -> Result<(), Box<dyn std::error::Error>> {
        // URLs contain colons but shouldn't trigger prefix parsing
        let result = parse_changelog_change("See https://example.com for details")?;
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "See https://example.com for details");
        assert!(!result.explicit, "URL colon means not explicit");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_conventional_commit_aliases() -> Result<(), Box<dyn std::error::Error>>
    {
        // feat → Added
        let r = parse_changelog_change("feat: new CLI flag")?;
        assert_eq!(r.category, ChangelogCategory::Added);

        // refactor → Changed
        let r = parse_changelog_change("refactor: extract module")?;
        assert_eq!(r.category, ChangelogCategory::Changed);

        // perf → Changed
        let r = parse_changelog_change("perf: optimize hot path")?;
        assert_eq!(r.category, ChangelogCategory::Changed);

        // test → Chore
        let r = parse_changelog_change("test: add snapshot tests")?;
        assert_eq!(r.category, ChangelogCategory::Chore);

        // docs → Chore
        let r = parse_changelog_change("docs: update README")?;
        assert_eq!(r.category, ChangelogCategory::Chore);

        // ci → Chore
        let r = parse_changelog_change("ci: fix pipeline")?;
        assert_eq!(r.category, ChangelogCategory::Chore);

        // build → Chore
        let r = parse_changelog_change("build: update dependencies")?;
        assert_eq!(r.category, ChangelogCategory::Chore);
        Ok(())
    }
}
