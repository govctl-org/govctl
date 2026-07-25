use super::*;

#[test]
fn test_list_filter_by_tag() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["adr", "new", "Tagged Decision"],
            &["adr", "new", "Untagged Decision"],
            &["adr", "edit", "ADR-0001", "tags", "--add", "caching"],
        ],
    )?;

    let output = run_commands(temp_dir.path(), &[&["adr", "list", "--tag", "caching"]])?;
    assert_tag_snapshot(
        "list_filter_by_tag",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_list_filter_multiple_tags() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["tag", "new", "performance"],
            &["tag", "new", "security"],
            &["adr", "new", "Multi-Tagged Decision"],
            &["adr", "edit", "ADR-0001", "tags", "--add", "caching"],
            &["adr", "edit", "ADR-0001", "tags", "--add", "performance"],
        ],
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "list", "--tag", "caching,performance"],
            &["adr", "list", "--tag", "caching,security"],
        ],
    )?;
    assert_tag_snapshot(
        "list_filter_multiple_tags",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}
