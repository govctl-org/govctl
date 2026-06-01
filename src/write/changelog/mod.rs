//! RFC changelog parsing and versioning helpers.
//!
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChangelogCategory, ChangelogEntry, RfcSpec};
use anyhow::Result;
use chrono::Local;
use semver::Version;

#[cfg(test)]
mod tests;

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
    let mut version = Version::parse(&rfc.version).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!("Invalid RFC version '{}': {err}", rfc.version),
            &rfc.rfc_id,
        )
    })?;

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
