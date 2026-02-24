//! Changelog and release workflow tests.

mod common;

use common::{normalize_output, run_commands, run_dynamic_commands, today};
use tempfile::TempDir;

/// Test the full changelog/release workflow:
/// 1. Initialize project
/// 2. Create work items with various categories
/// 3. Cut a release
/// 4. Create more unreleased work items
/// 5. Render changelog and test release command
#[test]
fn test_changelog_release_workflow() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let dir = temp_dir.path();
    let date = today();

    // Build work item IDs with actual date
    let wi1 = format!("WI-{}-001", date);
    let wi2 = format!("WI-{}-002", date);
    let wi3 = format!("WI-{}-003", date);
    let wi4 = format!("WI-{}-004", date);

    // Phase 1: Setup - Initialize and create first batch of work items
    let setup_commands: Vec<Vec<String>> = vec![
        vec!["init".to_string()],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Initial setup".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "add: Project scaffolding complete".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "add: Basic configuration".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "scaffolding".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "configuration".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi1.clone(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Bug fixes".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "fix: Memory leak in parser".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "fix: Crash on empty input".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "Memory leak".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "Crash".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi2.clone(),
            "done".to_string(),
        ],
        vec![
            "release".to_string(),
            "0.1.0".to_string(),
            "--date".to_string(),
            "2026-01-15".to_string(),
        ],
    ];

    let setup_output = run_dynamic_commands(dir, &setup_commands);
    insta::assert_snapshot!(
        "changelog_setup",
        normalize_output(&setup_output, dir, &date)
    );

    // Phase 2: Create unreleased work items for v0.2.0
    let unreleased_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "New features".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "add: User authentication".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "security: Password hashing".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "authentication".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "Password".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi3.clone(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "API changes".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "changed: Response format to JSON".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "deprecated: Legacy XML endpoint".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "removed: Obsolete v1 API".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "Response".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "Legacy".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "Obsolete".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi4.clone(),
            "done".to_string(),
        ],
    ];

    let unreleased_output = run_dynamic_commands(dir, &unreleased_commands);
    insta::assert_snapshot!(
        "changelog_unreleased",
        normalize_output(&unreleased_output, dir, &date)
    );

    // Phase 3: Test changelog rendering and release preview
    let changelog_output = run_commands(
        dir,
        &[
            &["status"],
            &["render", "changelog", "--dry-run"],
            &["release", "0.2.0", "--dry-run"],
        ],
    );
    insta::assert_snapshot!(
        "changelog_render",
        normalize_output(&changelog_output, dir, &date)
    );

    // Phase 4: Test error cases
    let error_output = run_commands(
        dir,
        &[&["release", "invalid-version"], &["release", "0.1.0"]],
    );
    insta::assert_snapshot!(
        "changelog_errors",
        normalize_output(&error_output, dir, &date)
    );
}

/// Test that changelog incremental rendering preserves ALL existing releases,
/// including those not in releases.toml, and preserves manual content.
/// Covers bugs fixed in WI-2026-02-25-001:
/// - Releases not in releases.toml were discarded
/// - Manual content was lost on re-render
/// - Version prefix matching (v0.3.0 vs 0.3.0)
/// - Inline refs used absolute paths
/// - Older versions were re-expanded
#[test]
fn test_changelog_preserves_existing_releases() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let dir = temp_dir.path();
    let date = today();

    // Build work item IDs with actual date
    let wi1 = format!("WI-{}-001", date);
    let wi2 = format!("WI-{}-002", date);

    // Phase 1: Initialize project
    let _ = run_commands(dir, &[&["init"]]);

    // Phase 2: Create work items and releases
    let setup_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Feature A".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "added: Feature A implementation".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "Feature A".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi1.clone(),
            "done".to_string(),
        ],
        vec![
            "release".to_string(),
            "0.2.0".to_string(),
            "--date".to_string(),
            "2026-01-15".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Feature B".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "added: Feature B implementation".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "Feature B".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi2.clone(),
            "done".to_string(),
        ],
        vec![
            "release".to_string(),
            "0.3.0".to_string(),
            "--date".to_string(),
            "2026-01-20".to_string(),
        ],
    ];
    let _ = run_dynamic_commands(dir, &setup_commands);

    // Phase 3: Create CHANGELOG with manual content and multiple versions
    // Include versions 0.3.0, 0.2.1, 0.2.0, 0.1.0 but releases.toml only has 0.3.0, 0.2.0
    let changelog_content = format!(
        r#"# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v0.3.0] - 2026-01-20

### Added

- Feature B implementation ({wi2})

### Changed

- **BREAKING**: Manual content in 0.3.0 that should be preserved
  - This tests that manual changes are not lost on re-render

## [0.2.1] - 2026-01-18

### Fixed

- Manual bug fix entry (no work item)
  - This tests releases not in releases.toml are preserved

## [v0.2.0] - 2026-01-15

### Added

- Feature A implementation ({wi1})

### Changed

- **BREAKING**: Manual content in 0.2.0
  - Tests preservation of existing releases
  - Tests inline ref: See [[ADR-0014]] for details

## [0.1.0] - 2026-01-10

### Added

- Initial release

"#,
        wi1 = wi1,
        wi2 = wi2
    );

    std::fs::write(dir.join("CHANGELOG.md"), &changelog_content)
        .expect("failed to write CHANGELOG.md");

    // Phase 4: Render changelog (should preserve all content)
    let render_output = run_commands(dir, &[&["render", "changelog"]]);

    // Verify the output
    let rendered =
        std::fs::read_to_string(dir.join("CHANGELOG.md")).expect("failed to read CHANGELOG.md");

    // Test 1: All versions should be preserved (even those not in releases.toml)
    // Check for version headers (support both X.Y.Z and vX.Y.Z formats)
    let has_030 = rendered.contains("## [0.3.0]") || rendered.contains("## [v0.3.0]");
    assert!(has_030, "0.3.0 should be preserved (v prefix version)");

    assert!(
        rendered.contains("## [0.2.1]"),
        "0.2.1 should be preserved (not in releases.toml)"
    );

    let has_020 = rendered.contains("## [0.2.0]") || rendered.contains("## [v0.2.0]");
    assert!(has_020, "0.2.0 should be preserved");

    assert!(
        rendered.contains("## [0.1.0]"),
        "0.1.0 should be preserved (not in releases.toml)"
    );

    // Test 2: Manual content should be preserved
    assert!(
        rendered.contains("BREAKING**: Manual content in 0.3.0"),
        "Manual content in 0.3.0 should be preserved"
    );
    assert!(
        rendered.contains("BREAKING**: Manual content in 0.2.0"),
        "Manual content in 0.2.0 should be preserved"
    );
    assert!(
        rendered.contains("Manual bug fix entry"),
        "Manual entry in 0.2.1 should be preserved"
    );

    // Test 3: Work item content should be present
    assert!(
        rendered.contains(&format!("Feature A implementation ({wi1})", wi1 = wi1)),
        "Feature A should be in changelog"
    );
    assert!(
        rendered.contains(&format!("Feature B implementation ({wi2})", wi2 = wi2)),
        "Feature B should be in changelog"
    );

    // Test 4: Inline refs in PRESERVED releases should stay as-is (not re-expanded)
    // The [[ADR-0014]] in 0.2.0 section was in original CHANGELOG, should remain unexpanded
    assert!(
        rendered.contains("[[ADR-0014]]"),
        "Inline refs in preserved releases should stay as-is (not re-expanded)"
    );
    // Test 4b: Inline refs should NOT use absolute paths anywhere
    assert!(
        !rendered.contains("/Users/"),
        "CHANGELOG should NOT contain absolute paths"
    );
    assert!(
        !rendered.contains("/home/"),
        "CHANGELOG should NOT contain absolute paths"
    );

    // Test 5: Versions should be in semver descending order
    let lines: Vec<&str> = rendered.lines().collect();
    let version_indices: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("## [") && !line.contains("Unreleased"))
        .map(|(i, line)| (i, *line))
        .collect();

    // Should have 4 versions
    assert_eq!(version_indices.len(), 4, "Should have 4 release versions");

    // First version should be 0.3.0 (newest)
    assert!(
        version_indices[0].1.contains("0.3.0"),
        "First version should be 0.3.0"
    );

    insta::assert_snapshot!(
        "changelog_preserve_all",
        normalize_output(&render_output, dir, &date)
    );
}

/// Test that inline refs in NEW releases are expanded with relative paths.
/// This tests bug fix: inline refs now use relative 'docs/' path instead of absolute.
#[test]
fn test_changelog_inline_refs_use_relative_paths() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let dir = temp_dir.path();
    let date = today();

    let wi1 = format!("WI-{}-001", date);

    // Initialize and create work item with inline ref
    let _ = run_commands(dir, &[&["init"]]);

    let setup_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Feature with ref".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "added: Feature per [[ADR-0014]] requirements".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "Feature per".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi1.clone(),
            "done".to_string(),
        ],
        vec![
            "release".to_string(),
            "0.1.0".to_string(),
            "--date".to_string(),
            "2026-01-15".to_string(),
        ],
    ];
    let _ = run_dynamic_commands(dir, &setup_commands);

    // Render changelog
    let _ = run_commands(dir, &[&["render", "changelog"]]);

    // Verify inline ref is expanded with relative path
    let rendered =
        std::fs::read_to_string(dir.join("CHANGELOG.md")).expect("failed to read CHANGELOG.md");

    // Inline ref should be expanded to relative path
    assert!(
        rendered.contains("[ADR-0014](docs/adr/ADR-0014.md)"),
        "Inline refs in new releases should be expanded with relative 'docs/' path"
    );

    // Should NOT contain absolute paths
    assert!(
        !rendered.contains("/Users/"),
        "New release inline refs should NOT use absolute paths"
    );
    assert!(
        !rendered.contains("/home/"),
        "New release inline refs should NOT use absolute paths"
    );
}
