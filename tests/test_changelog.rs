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
fn test_changelog_release_workflow() -> common::TestResult {
    let temp_dir = TempDir::new()?;
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

    let setup_output = run_dynamic_commands(dir, &setup_commands)?;
    insta::assert_snapshot!(
        "changelog_setup",
        normalize_output(&setup_output, dir, &date)?
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

    let unreleased_output = run_dynamic_commands(dir, &unreleased_commands)?;
    insta::assert_snapshot!(
        "changelog_unreleased",
        normalize_output(&unreleased_output, dir, &date)?
    );

    // Phase 3: Test changelog rendering and release preview
    let changelog_output = run_commands(
        dir,
        &[
            &["status"],
            &["render", "changelog", "--dry-run"],
            &["release", "0.2.0", "--dry-run"],
        ],
    )?;
    insta::assert_snapshot!(
        "changelog_render",
        normalize_output(&changelog_output, dir, &date)?
    );

    // Phase 4: Test error cases
    let error_output = run_commands(
        dir,
        &[&["release", "invalid-version"], &["release", "0.1.0"]],
    )?;
    insta::assert_snapshot!(
        "changelog_errors",
        normalize_output(&error_output, dir, &date)?
    );
    Ok(())
}

/// Integration test: changelog preservation across the full adoption lifecycle.
///
/// Phase 1 — Brownfield: legacy CHANGELOG pre-dates govctl. First govctl
/// release is cut and rendered on top of it. Verifies legacy versions and
/// content survive, inline refs in new releases use relative paths, and
/// v-prefix versions are handled.
///
/// Phase 2 — Steady state: user manually edits the rendered CHANGELOG
/// (adds a patch release not in releases.toml, appends content to an
/// existing section, adds inline refs). A second govctl release is cut
/// and re-rendered. Verifies manual edits, hand-written versions, and
/// unexpanded inline refs are all preserved.
#[test]
fn test_changelog_preservation_lifecycle() -> common::TestResult {
    let temp_dir = TempDir::new()?;
    let dir = temp_dir.path();
    let date = today();

    let wi1 = format!("WI-{}-001", date);
    let wi2 = format!("WI-{}-002", date);

    // ── Phase 1: Brownfield adoption ──────────────────────────────────

    let _ = run_commands(dir, &[&["init"]])?;

    // Legacy CHANGELOG: no [Unreleased], no govctl artifacts, mixed v-prefix
    let legacy_changelog = "\
# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0] - 2025-06-01

### Added

- Widget subsystem with drag-and-drop support
- Keyboard navigation for accessibility

### Fixed

- Race condition in event dispatcher (#42)

## [v0.8.0] - 2025-03-15

### Changed

- Migrated from Webpack to Vite
- Upgraded tokio to 1.28

### Deprecated

- Legacy plugin API (use v2 hooks instead)

## [0.7.0] - 2025-01-10

### Added

- Initial public release
- REST API with OpenAPI spec
";
    std::fs::write(dir.join("CHANGELOG.md"), legacy_changelog)?;

    // First govctl work item — includes an inline ref
    let phase1_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Govctl adoption".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "added: Governance structure per [[ADR-0014]]".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "Governance".to_string(),
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
            "1.0.0".to_string(),
            "--date".to_string(),
            "2026-03-01".to_string(),
        ],
    ];
    let _ = run_dynamic_commands(dir, &phase1_commands)?;

    let _ = run_commands(dir, &[&["render", "changelog"]])?;
    let rendered = std::fs::read_to_string(dir.join("CHANGELOG.md"))?;

    // New govctl release appears
    assert!(rendered.contains("## [1.0.0]"), "1.0.0 should be present");
    assert!(
        rendered.contains("Governance structure"),
        "work item content should appear in 1.0.0"
    );

    // Inline ref in NEW release expanded to relative path
    assert!(
        rendered.contains("[ADR-0014](docs/adr/ADR-0014.md)"),
        "inline refs in new releases should expand to relative docs/ path"
    );
    assert!(
        !rendered.contains("/Users/"),
        "phase 1: no absolute paths (macOS)"
    );
    assert!(
        !rendered.contains("/home/"),
        "phase 1: no absolute paths (Linux)"
    );

    // Legacy versions preserved (handle v-prefix)
    assert!(rendered.contains("## [0.9.0]"), "legacy 0.9.0 preserved");
    let has_080 = rendered.contains("## [0.8.0]") || rendered.contains("## [v0.8.0]");
    assert!(has_080, "legacy 0.8.0 preserved (v-prefix ok)");
    assert!(rendered.contains("## [0.7.0]"), "legacy 0.7.0 preserved");

    // Legacy content intact
    assert!(rendered.contains("Widget subsystem with drag-and-drop support"));
    assert!(rendered.contains("Race condition in event dispatcher (#42)"));
    assert!(rendered.contains("Migrated from Webpack to Vite"));
    assert!(rendered.contains("Legacy plugin API (use v2 hooks instead)"));
    assert!(rendered.contains("Initial public release"));

    // Semver descending
    let versions_p1: Vec<&str> = rendered
        .lines()
        .filter(|l| l.starts_with("## [") && !l.contains("Unreleased"))
        .collect();
    assert_eq!(versions_p1.len(), 4, "phase 1: 4 versions");
    assert!(versions_p1[0].contains("1.0.0"), "newest first");
    assert!(versions_p1[3].contains("0.7.0"), "oldest last");

    insta::assert_snapshot!(
        "changelog_lifecycle_phase1",
        normalize_output(&rendered, dir, &date)?
    );

    // ── Phase 2: Manual edits + second release ───────────────────────

    // Simulate user manually editing the rendered CHANGELOG:
    // - Insert a hand-written 1.0.1 patch (not in releases.toml)
    //   with an inline ref that should stay unexpanded on re-render
    // - Add manual BREAKING note to the 1.0.0 section
    let phase2_changelog = format!(
        r#"# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.1] - 2026-03-05

### Fixed

- Hotfix for config loading edge case (see [[ADR-0014]])

## [1.0.0] - 2026-03-01

### Added

- Governance structure per [ADR-0014](docs/adr/ADR-0014.md) ({wi1})

### Changed

- **BREAKING**: Manual note added after initial render
  - This content was hand-edited by a human

## [0.9.0] - 2025-06-01

### Added

- Widget subsystem with drag-and-drop support
- Keyboard navigation for accessibility

### Fixed

- Race condition in event dispatcher (#42)

## [v0.8.0] - 2025-03-15

### Changed

- Migrated from Webpack to Vite
- Upgraded tokio to 1.28

### Deprecated

- Legacy plugin API (use v2 hooks instead)

## [0.7.0] - 2025-01-10

### Added

- Initial public release
- REST API with OpenAPI spec
"#,
        wi1 = wi1,
    );
    std::fs::write(dir.join("CHANGELOG.md"), &phase2_changelog)?;

    // Second govctl release
    let phase2_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Breaking changes".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "changed: Response format to JSON".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "Response".to_string(),
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
            "2.0.0".to_string(),
            "--date".to_string(),
            "2026-03-14".to_string(),
        ],
    ];
    let _ = run_dynamic_commands(dir, &phase2_commands)?;

    let _ = run_commands(dir, &[&["render", "changelog"]])?;
    let rendered = std::fs::read_to_string(dir.join("CHANGELOG.md"))?;

    // New release appears
    assert!(rendered.contains("## [2.0.0]"), "2.0.0 should be present");
    assert!(
        rendered.contains("Response format to JSON"),
        "work item content should appear in 2.0.0"
    );

    // Hand-written 1.0.1 preserved (not in releases.toml)
    assert!(
        rendered.contains("## [1.0.1]"),
        "hand-written 1.0.1 preserved"
    );
    assert!(
        rendered.contains("Hotfix for config loading edge case"),
        "1.0.1 content preserved"
    );

    // Inline ref in PRESERVED section stays as-is
    assert!(
        rendered.contains("[[ADR-0014]]"),
        "inline refs in preserved sections should NOT be expanded"
    );

    // Manual content in 1.0.0 preserved
    assert!(
        rendered.contains("BREAKING**: Manual note added after initial render"),
        "manual BREAKING content in 1.0.0 preserved"
    );
    assert!(
        rendered.contains("hand-edited by a human"),
        "manual sub-bullet in 1.0.0 preserved"
    );

    // All legacy versions still present
    assert!(rendered.contains("## [0.9.0]"), "legacy 0.9.0 still there");
    let has_080 = rendered.contains("## [0.8.0]") || rendered.contains("## [v0.8.0]");
    assert!(has_080, "legacy 0.8.0 still there");
    assert!(rendered.contains("## [0.7.0]"), "legacy 0.7.0 still there");

    // No absolute paths
    assert!(
        !rendered.contains("/Users/"),
        "phase 2: no absolute paths (macOS)"
    );
    assert!(
        !rendered.contains("/home/"),
        "phase 2: no absolute paths (Linux)"
    );

    // Semver descending: 2.0.0 > 1.0.1 > 1.0.0 > 0.9.0 > 0.8.0 > 0.7.0
    let versions_p2: Vec<&str> = rendered
        .lines()
        .filter(|l| l.starts_with("## [") && !l.contains("Unreleased"))
        .collect();
    assert_eq!(versions_p2.len(), 6, "phase 2: 6 versions");
    assert!(versions_p2[0].contains("2.0.0"), "newest first");
    assert!(
        versions_p2[1].contains("1.0.1"),
        "hand-written patch second"
    );
    assert!(
        versions_p2[2].contains("1.0.0"),
        "first govctl release third"
    );
    assert!(versions_p2[5].contains("0.7.0"), "oldest last");

    insta::assert_snapshot!(
        "changelog_lifecycle_phase2",
        normalize_output(&rendered, dir, &date)?
    );
    Ok(())
}
