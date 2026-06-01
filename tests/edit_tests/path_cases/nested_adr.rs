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
