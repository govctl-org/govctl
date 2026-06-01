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
fn test_parse_changelog_conventional_commit_aliases() -> Result<(), Box<dyn std::error::Error>> {
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
