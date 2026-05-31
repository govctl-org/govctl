// ============================================================================
// Field Alias Tests
// ============================================================================

#[test]
fn test_field_alias_ac() -> common::TestResult {
    // 'ac' should resolve to 'acceptance_criteria'
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "ac",
                "add: Test criterion",
            ],
            &["work", "get", &format!("WI-{}-001", date), "ac"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_desc() -> common::TestResult {
    // 'desc' should resolve to 'description'
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "desc",
                "A description",
            ],
            &["work", "get", &format!("WI-{}-001", date), "desc"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_desc_under_legacy_prefix() -> common::TestResult {
    // content.desc should resolve to description on work items
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "content.desc",
                "Legacy-prefixed description",
            ],
            &["work", "get", &format!("WI-{}-001", date), "description"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_field_alias_desc_not_global_on_adr() -> common::TestResult {
    // desc is not a valid ADR root field alias and should not be rewritten globally
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Alias Scope"],
            &["adr", "set", "ADR-0001", "desc", "nope"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_tick_rejects_nested_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Nested Tick"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "ac[0].text",
                "Criterion 1",
                "-s",
                "done",
            ],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

// ============================================================================
// Nested Path Tests (ADR-0029)
// ============================================================================

#[test]
fn test_adr_get_nested_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Path Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Use traits",
                "--pro",
                "Flexible",
                "--pro",
                "Reusable",
                "--con",
                "Complex",
            ],
            &["adr", "get", "ADR-0001", "alt[0].text"],
            &["adr", "get", "ADR-0001", "alt[0].pros"],
            &["adr", "get", "ADR-0001", "alt[0].pros[0]"],
            &["adr", "get", "ADR-0001", "alt[0].cons"],
            &["adr", "get", "ADR-0001", "alternatives[0]"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_set_nested_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Set Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option A",
                "--pro",
                "Fast",
                "--con",
                "Fragile",
            ],
            &["adr", "set", "ADR-0001", "alt[0].text", "Option A Revised"],
            &["adr", "get", "ADR-0001", "alt[0].text"],
            &["adr", "set", "ADR-0001", "alt[0].pros[0]", "Very fast"],
            &["adr", "get", "ADR-0001", "alt[0].pros[0]"],
            &[
                "adr",
                "set",
                "ADR-0001",
                "alt[0].rejection_reason",
                "Superseded by Option B",
            ],
            &["adr", "get", "ADR-0001", "alt[0].rejection_reason"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_add_nested_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Add Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Cheap",
            ],
            &["adr", "add", "ADR-0001", "alt[0].pros", "Reliable"],
            &["adr", "get", "ADR-0001", "alt[0].pros"],
            &["adr", "add", "ADR-0001", "alt[0].cons", "Slow"],
            &["adr", "get", "ADR-0001", "alt[0].cons"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_nested_path_rejects_extra_segments() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Depth Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Fast",
            ],
            &["adr", "get", "ADR-0001", "alt[0].pros[0].oops"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_add_nested_path_rejects_indexed_terminal() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Indexed Add Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Fast",
            ],
            &["adr", "add", "ADR-0001", "alt[0].pros[999]", "Ignored"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_get_nested_scalar_rejects_index() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Scalar Index Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Option X",
                "--pro",
                "Fast",
            ],
            &["adr", "get", "ADR-0001", "alt[0].text[0]"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_remove_nested_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Remove Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Opt1",
                "--pro",
                "Good",
                "--pro",
                "Great",
                "--con",
                "Bad",
            ],
            // Remove by sub-index
            &["adr", "remove", "ADR-0001", "alt[0].pros[0]"],
            &["adr", "get", "ADR-0001", "alt[0].pros"],
            // Remove con by pattern match (no terminal index)
            &["adr", "remove", "ADR-0001", "alt[0].cons", "Bad"],
            &["adr", "get", "ADR-0001", "alt[0].cons"],
            // Remove entire alternative
            &["adr", "remove", "ADR-0001", "alt[0]"],
            &["adr", "get", "ADR-0001", "alternatives"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_get_nested_scalar_rejects_index() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let wi_id = format!("WI-{}-001", date);
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Work Scalar Index Test"],
            &[
                "work",
                "add",
                &wi_id,
                "acceptance_criteria",
                "add: Did something",
            ],
            &["work", "get", &wi_id, "acceptance_criteria[0].text[0]"],
        ],
    )?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    assert!(
        normalized.contains("error[E0817]: Cannot index into non-list field 'text'"),
        "output: {normalized}"
    );
    Ok(())
}

#[test]
fn test_work_journal_legacy_entries_are_read_only() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let wi_id = format!("WI-{}-001", date);
    let work_path = temp_dir
        .path()
        .join("gov")
        .join("work")
        .join(format!("{}-legacy-journal-test.toml", date));
    let output = run_commands(temp_dir.path(), &[&["work", "new", "Legacy Journal Test"]])?;
    assert!(output.contains("Created work item"), "output: {output}");

    let mut content = std::fs::read_to_string(&work_path)?;
    content.push_str(
        r#"

[[content.journal]]
date = "2026-02-22"
content = "Keep this history"
"#,
    );
    std::fs::write(&work_path, content)?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "get", &wi_id, "journal[0].content"],
            &[
                "work",
                "edit",
                &wi_id,
                "journal[0].content",
                "--set",
                "Changed history",
            ],
            &["work", "remove", &wi_id, "journal", "--all"],
            &["work", "show", &wi_id],
        ],
    )?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    assert!(
        normalized.contains("Keep this history"),
        "legacy journal should still render via work show: {normalized}"
    );
    assert_eq!(
        normalized.matches("error[E0803]").count(),
        3,
        "legacy journal get/set/remove field operations should be rejected: {normalized}"
    );
    assert!(
        normalized.contains("## Journal")
            && normalized.contains("Legacy execution history")
            && normalized.contains("loop state"),
        "legacy journal should render with neutral migration guidance: {normalized}"
    );
    let persisted = std::fs::read_to_string(&work_path)?;
    assert!(
        !persisted.contains("Changed history"),
        "legacy journal should not be mutated: {persisted}"
    );
    Ok(())
}

#[test]
fn test_adr_remove_nested_path_requires_selector() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Selector Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Opt1",
                "--con",
                "Bad",
            ],
            &["adr", "remove", "ADR-0001", "alt[0].cons"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_adr_edit_tick_updates_alternative_root() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Tick Root Test"],
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--tick",
                "accepted",
                "--at",
                "0",
            ],
            &["adr", "get", "ADR-0001", "alternatives"],
        ],
    )?;
    assert!(
        output.contains("Marked 'Option A' as accepted"),
        "output: {}",
        output
    );
    assert!(output.contains("[accepted] Option A"), "output: {}", output);
    Ok(())
}

#[test]
fn test_adr_edit_tick_updates_indexed_alternative_item() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Indexed Tick Test"],
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &["adr", "edit", "ADR-0001", "alt[0]", "--tick", "accepted"],
            &["adr", "get", "ADR-0001", "alternatives[0].status"],
        ],
    )?;
    assert!(
        output.contains("Marked 'Option A' as accepted"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl adr get ADR-0001 alternatives[0].status\naccepted"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_adr_edit_tick_rejects_work_item_status_names() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Invalid Tick Test"],
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &[
                "adr",
                "edit",
                "ADR-0001",
                "alternatives",
                "--tick",
                "done",
                "--at",
                "0",
            ],
        ],
    )?;
    assert!(output.contains("error[E0820]"), "output: {}", output);
    assert!(
        output.contains("ADR tick status must be one of: accepted, considered, rejected"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_remove_indexed_path_conflict() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Conflict Test"],
            &[
                "adr",
                "add",
                "ADR-0001",
                "alternatives",
                "Opt1",
                "--con",
                "Bad",
            ],
            // Indexed path + --exact should produce E0818
            &[
                "adr",
                "remove",
                "ADR-0001",
                "alt[0].cons[0]",
                "--exact",
                "Bad",
            ],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_path_backward_compat() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Compat Test"],
            // Implements [[ADR-0042]]: must have alternatives before setting decision
            &["adr", "add", "ADR-0001", "alternatives", "Option A"],
            &["adr", "add", "ADR-0001", "alternatives", "Option B"],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "0",
                "-s",
                "accepted",
            ],
            &[
                "adr",
                "tick",
                "ADR-0001",
                "alternatives",
                "--at",
                "1",
                "-s",
                "rejected",
            ],
            // Legacy dotted paths should still work
            &[
                "adr",
                "set",
                "ADR-0001",
                "content.decision",
                "A dotted decision",
            ],
            &["adr", "get", "ADR-0001", "content.decision"],
            &["adr", "set", "ADR-0001", "govctl.title", "Compat Title"],
            &["adr", "get", "ADR-0001", "govctl.title"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
