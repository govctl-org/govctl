use super::*;

#[test]
fn test_tag_new() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let output = run_commands(
        temp_dir.path(),
        &[&["tag", "new", "caching"], &["tag", "list"]],
    )?;
    assert_tag_snapshot(
        "tag_new",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_tag_new_duplicate() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let output = run_commands(
        temp_dir.path(),
        &[&["tag", "new", "caching"], &["tag", "new", "caching"]],
    )?;
    assert_tag_snapshot(
        "tag_new_duplicate",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_tag_new_invalid_format() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let output = run_commands(temp_dir.path(), &[&["tag", "new", "UPPER"]])?;
    assert_tag_snapshot(
        "tag_new_invalid_format",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_tag_delete() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["tag", "delete", "caching"],
            &["tag", "list"],
        ],
    )?;
    assert_tag_snapshot(
        "tag_delete",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_tag_delete_referenced() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
            &["tag", "delete", "caching"],
        ],
    )?;
    assert_tag_snapshot(
        "tag_delete_referenced",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_tag_list_plain_and_json_output() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["tag", "new", "security"],
            &["adr", "new", "Tagged Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
        ],
    )?;

    let plain = normalize_output(
        &run_commands(temp_dir.path(), &[&["tag", "list", "-o", "plain"]])?,
        temp_dir.path(),
        &date,
    )?;
    assert!(plain.contains("caching\t1"), "{plain}");
    assert!(plain.contains("security\t0"), "{plain}");

    let json = normalize_output(
        &run_commands(temp_dir.path(), &[&["tag", "list", "-o", "json"]])?,
        temp_dir.path(),
        &date,
    )?;
    assert!(json.contains(r#""tag": "caching""#), "{json}");
    assert!(json.contains(r#""usage": 1"#), "{json}");
    assert!(json.contains(r#""tag": "security""#), "{json}");
    assert!(json.contains(r#""usage": 0"#), "{json}");
    Ok(())
}
