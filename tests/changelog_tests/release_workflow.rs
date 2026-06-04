use super::*;

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
        command(&["init"]),
        work_new_active("Initial setup"),
        work_add_acceptance(&wi1, "add: Project scaffolding complete"),
        work_add_acceptance(&wi1, "add: Basic configuration"),
        work_tick_acceptance_done(&wi1, "scaffolding"),
        work_tick_acceptance_done(&wi1, "configuration"),
        work_move_done(&wi1),
        work_new_active("Bug fixes"),
        work_add_acceptance(&wi2, "fix: Memory leak in parser"),
        work_add_acceptance(&wi2, "fix: Crash on empty input"),
        work_tick_acceptance_done(&wi2, "Memory leak"),
        work_tick_acceptance_done(&wi2, "Crash"),
        work_move_done(&wi2),
        command(&["release", "0.1.0", "--date", "2026-01-15"]),
    ];

    let setup_output = run_dynamic_commands(dir, &setup_commands)?;
    assert_changelog_snapshot!(
        "changelog_setup",
        normalize_output(&setup_output, dir, &date)?
    );

    // Phase 2: Create unreleased work items for v0.2.0
    let unreleased_commands: Vec<Vec<String>> = vec![
        work_new_active("New features"),
        work_add_acceptance(&wi3, "add: User authentication"),
        work_add_acceptance(&wi3, "security: Password hashing"),
        work_tick_acceptance_done(&wi3, "authentication"),
        work_tick_acceptance_done(&wi3, "Password"),
        work_move_done(&wi3),
        work_new_active("API changes"),
        work_add_acceptance(&wi4, "changed: Response format to JSON"),
        work_add_acceptance(&wi4, "deprecated: Legacy XML endpoint"),
        work_add_acceptance(&wi4, "removed: Obsolete v1 API"),
        work_tick_acceptance_done(&wi4, "Response"),
        work_tick_acceptance_done(&wi4, "Legacy"),
        work_tick_acceptance_done(&wi4, "Obsolete"),
        work_move_done(&wi4),
    ];

    let unreleased_output = run_dynamic_commands(dir, &unreleased_commands)?;
    assert_changelog_snapshot!(
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
    assert_changelog_snapshot!(
        "changelog_render",
        normalize_output(&changelog_output, dir, &date)?
    );

    // Phase 4: Test error cases
    let error_output = run_commands(
        dir,
        &[&["release", "invalid-version"], &["release", "0.1.0"]],
    )?;
    assert_changelog_snapshot!(
        "changelog_errors",
        normalize_output(&error_output, dir, &date)?
    );
    Ok(())
}
