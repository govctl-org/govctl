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
