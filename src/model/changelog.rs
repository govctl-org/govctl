use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

/// Changelog entry for RFC versioning (Keep a Changelog format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    /// Optional freeform notes for this release
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// New features added
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<String>,
    /// Changes to existing functionality
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<String>,
    /// Features marked for removal in upcoming releases
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deprecated: Vec<String>,
    /// Features removed in this release
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// Bug fixes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed: Vec<String>,
    /// Security-related changes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<String>,
}

/// Changelog category for Keep a Changelog format.
/// Used for both RFC changelog entries and work item acceptance criteria.
/// Per [[ADR-0012]] and [[ADR-0013]].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, AsRefStr, ValueEnum,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ChangelogCategory {
    #[default]
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
    /// Internal/housekeeping items - excluded from rendered changelog
    Chore,
}

impl ChangelogCategory {
    /// Canonical category prefixes shown in error messages and docs.
    /// All verb forms for consistency with imperative acceptance criteria.
    pub const VALID_PREFIXES: &'static [&'static str] = &[
        "add",
        "fix",
        "change",
        "remove",
        "deprecate",
        "security",
        "chore",
    ];

    pub const RELEASE_CHANGELOG_SECTIONS: &'static [(Self, &'static str)] = &[
        (Self::Added, "Added"),
        (Self::Changed, "Changed"),
        (Self::Deprecated, "Deprecated"),
        (Self::Removed, "Removed"),
        (Self::Fixed, "Fixed"),
        (Self::Security, "Security"),
    ];

    /// Parse a prefix string into a category.
    ///
    /// Accepts canonical Keep-a-Changelog names, verb forms, and
    /// conventional-commit aliases so agents and humans can use
    /// whichever style they reach for first.
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix.to_lowercase().as_str() {
            // Added: new features and capabilities
            "add" | "added" | "feat" | "feature" => Some(Self::Added),
            // Changed: modifications to existing behavior
            "changed" | "change" | "refactor" | "perf" => Some(Self::Changed),
            // Deprecated: features marked for future removal
            "deprecated" | "deprecate" => Some(Self::Deprecated),
            // Removed: deleted features
            "removed" | "remove" => Some(Self::Removed),
            // Fixed: bug fixes
            "fix" | "fixed" => Some(Self::Fixed),
            // Security: vulnerability fixes
            "security" | "sec" => Some(Self::Security),
            // Chore: internal tasks excluded from changelog
            "chore" | "internal" | "test" | "tests" | "doc" | "docs" | "ci" | "build" => {
                Some(Self::Chore)
            }
            _ => None,
        }
    }

    pub fn from_rendered_prefix(prefix: &str) -> Option<Self> {
        match prefix.to_lowercase().as_str() {
            "added" => Some(Self::Added),
            "changed" => Some(Self::Changed),
            "deprecated" => Some(Self::Deprecated),
            "removed" => Some(Self::Removed),
            "fixed" => Some(Self::Fixed),
            "security" => Some(Self::Security),
            "chore" => Some(Self::Chore),
            _ => None,
        }
    }

    pub fn strip_rendered_prefix(text: &str) -> Option<&str> {
        let (prefix, rest) = text.split_once(':')?;
        Self::from_rendered_prefix(prefix).map(|_| rest.trim_start())
    }
}
