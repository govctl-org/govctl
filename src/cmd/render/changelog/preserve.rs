use std::collections::BTreeMap;

use super::sections::CHANGELOG_HEADER;

pub(super) struct ExistingChangelog {
    pub header: String,
    pub releases: BTreeMap<String, String>,
}

pub(super) fn split_existing_changelog(existing: &str) -> ExistingChangelog {
    let (header, released) = split_header_and_released_sections(existing);
    ExistingChangelog {
        header,
        releases: release_sections_by_version(&released),
    }
}

pub(super) fn contains_version_variant(releases: &BTreeMap<String, String>, version: &str) -> bool {
    version_variants(version)
        .iter()
        .any(|variant| releases.contains_key(variant))
}

pub(super) fn versions_newest_first(releases: &BTreeMap<String, String>) -> Vec<String> {
    let mut versions: Vec<String> = releases.keys().cloned().collect();
    versions.sort_by(|a, b| {
        let a_parts = version_parts(a);
        let b_parts = version_parts(b);
        b_parts.cmp(&a_parts)
    });
    versions
}

fn split_header_and_released_sections(existing: &str) -> (String, String) {
    const UNRELEASED_HEADER: &str = "## [Unreleased]";
    const RELEASE_PATTERN: &str = "\n## [";

    if existing.is_empty() {
        return (CHANGELOG_HEADER.to_string(), String::new());
    }

    if let Some(unreleased_pos) = existing.find(UNRELEASED_HEADER) {
        let header = existing[..unreleased_pos].to_string();
        let after_unreleased = &existing[unreleased_pos + UNRELEASED_HEADER.len()..];
        let released = if let Some(pos) = after_unreleased.find(RELEASE_PATTERN) {
            after_unreleased[pos + 1..].to_string()
        } else {
            String::new()
        };
        return (header, released);
    }

    if let Some(first_release_pos) = existing.find(RELEASE_PATTERN) {
        let header = existing[..first_release_pos + 1].to_string();
        let released = existing[first_release_pos + 1..].to_string();
        return (header, released);
    }

    (existing.to_string(), String::new())
}

fn release_sections_by_version(content: &str) -> BTreeMap<String, String> {
    let mut releases = BTreeMap::new();
    if content.is_empty() {
        return releases;
    }

    let mut current_pos = 0;
    while current_pos < content.len() {
        if !content[current_pos..].starts_with("## [") {
            break;
        }

        let rest = &content[current_pos..];
        let section_end = rest[4..]
            .find("\n## [")
            .map(|position| position + 4 + 1)
            .unwrap_or(rest.len());
        let section = &rest[..section_end];

        let version = if let Some(bracket_end) = section.find(']') {
            section[4..bracket_end].to_string()
        } else {
            current_pos += section.len();
            continue;
        };

        releases.insert(version, section.to_string());
        current_pos += section.len();
    }

    releases
}

fn version_variants(version: &str) -> Vec<String> {
    if let Some(without_prefix) = version.strip_prefix('v') {
        vec![version.to_string(), without_prefix.to_string()]
    } else {
        vec![version.to_string(), format!("v{}", version)]
    }
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .strip_prefix('v')
        .unwrap_or(version)
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect()
}
