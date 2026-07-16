//! RFC changelog parsing and versioning helpers.
//!
//! Implements [[ADR-0012]] prefix-based changelog category parsing.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{ChangelogCategory, ChangelogEntry, RfcSpec};
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
pub fn parse_changelog_change(change: &str) -> DiagnosticResult<ParsedChange> {
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
                    ));
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
                ));
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
pub fn bump_rfc_version(
    rfc: &mut RfcSpec,
    level: BumpLevel,
    summary: &str,
) -> DiagnosticResult<String> {
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
pub fn add_changelog_change(rfc: &mut RfcSpec, change: &str) -> DiagnosticResult<()> {
    let parsed = parse_changelog_change(change)?;

    let entry = current_changelog_entry_mut(rfc)?;
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
            ));
        }
    }
    Ok(())
}

/// Resolve the unique changelog entry for the RFC's current version.
///
/// Implements [[RFC-0000:C-RFC-DEF]].
pub fn current_changelog_entry(rfc: &RfcSpec) -> DiagnosticResult<&ChangelogEntry> {
    let index = current_changelog_index(rfc)?;
    Ok(&rfc.changelog[index])
}

/// Resolve the unique mutable changelog entry for the RFC's current version.
pub fn current_changelog_entry_mut(rfc: &mut RfcSpec) -> DiagnosticResult<&mut ChangelogEntry> {
    let index = current_changelog_index(rfc)?;
    Ok(&mut rfc.changelog[index])
}

fn current_changelog_index(rfc: &RfcSpec) -> DiagnosticResult<usize> {
    let matching: Vec<_> = rfc
        .changelog
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| (entry.version == rfc.version).then_some(index))
        .collect();

    if let [index] = matching.as_slice() {
        return Ok(*index);
    }

    let code = if matching.is_empty() {
        DiagnosticCode::E0111RfcNoChangelog
    } else {
        DiagnosticCode::E0115RfcCurrentChangelogInvalid
    };
    Err(Diagnostic::new(
        code,
        format!(
            "RFC must contain exactly one changelog entry for current version {} (found {})",
            rfc.version,
            matching.len()
        ),
        &rfc.rfc_id,
    ))
}

/// Get today's date in ISO format
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}
