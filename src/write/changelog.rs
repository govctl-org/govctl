//! RFC changelog parsing and versioning helpers.
//!
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChangelogCategory, ChangelogEntry, RfcSpec};
use anyhow::{Context, Result};
use chrono::Local;
use semver::Version;

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
/// - `fix: memory leak` -> Fixed category, "memory leak"
/// - `security: patched CVE` -> Security category, "patched CVE"
/// - `just a change` -> Added category (default), "just a change"
///
/// Returns error if prefix is present but invalid.
pub fn parse_changelog_change(change: &str) -> Result<ParsedChange> {
    if let Some(colon_pos) = change.find(':') {
        let prefix = change[..colon_pos].trim();
        let message = change[colon_pos + 1..].trim();

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

    Ok(ParsedChange {
        category: ChangelogCategory::Added,
        message: change.trim().to_string(),
        explicit: false,
    })
}

/// Version bump level
#[derive(Debug, Clone, Copy)]
pub enum BumpLevel {
    Patch,
    Minor,
    Major,
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
/// - `fix: message` -> fixed category
/// - `security: message` -> security category
/// - `message` (no prefix) -> added category
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ChangelogCategory;

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
        let result = parse_changelog_change("See https://example.com for details")?;
        assert_eq!(result.category, ChangelogCategory::Added);
        assert_eq!(result.message, "See https://example.com for details");
        assert!(!result.explicit, "URL colon means not explicit");
        Ok(())
    }

    #[test]
    fn test_parse_changelog_conventional_commit_aliases() -> Result<(), Box<dyn std::error::Error>>
    {
        let r = parse_changelog_change("feat: new CLI flag")?;
        assert_eq!(r.category, ChangelogCategory::Added);

        let r = parse_changelog_change("refactor: extract module")?;
        assert_eq!(r.category, ChangelogCategory::Changed);

        let r = parse_changelog_change("perf: optimize hot path")?;
        assert_eq!(r.category, ChangelogCategory::Changed);

        let r = parse_changelog_change("test: add snapshot tests")?;
        assert_eq!(r.category, ChangelogCategory::Chore);

        let r = parse_changelog_change("docs: update README")?;
        assert_eq!(r.category, ChangelogCategory::Chore);

        let r = parse_changelog_change("ci: fix pipeline")?;
        assert_eq!(r.category, ChangelogCategory::Chore);

        let r = parse_changelog_change("build: update dependencies")?;
        assert_eq!(r.category, ChangelogCategory::Chore);
        Ok(())
    }
}
