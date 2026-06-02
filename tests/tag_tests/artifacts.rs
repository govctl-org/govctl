use super::*;

#[test]
fn test_artifact_add_tag() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let output = run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "caching"],
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
            &["adr", "get", "ADR-0001", "tags"],
        ],
    )?;
    assert_tag_snapshot(
        "artifact_add_tag",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_artifact_add_unregistered_tag() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();

    register_tags(temp_dir.path(), &["registered"])?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "nonexistent"],
        ],
    )?;
    assert_tag_snapshot(
        "artifact_add_unregistered_tag",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_check_rejects_unknown_tag() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();

    register_tags(temp_dir.path(), &["allowed-tag"])?;

    run_commands(temp_dir.path(), &[&["adr", "new", "Test Decision"]])?;

    let adr_dir = temp_dir.path().join("gov/adr");
    let adr_path = fs::read_dir(&adr_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("ADR-0001") && n.ends_with(".toml"))
                .unwrap_or(false)
        })
        .ok_or("ADR-0001 file not found in gov/adr")?;

    let content = fs::read_to_string(&adr_path)?;
    let mut doc: toml::Table = toml::from_str(&content)?;
    let govctl = doc
        .get_mut("govctl")
        .and_then(|v| v.as_table_mut())
        .ok_or("[govctl] table must exist in ADR TOML")?;
    govctl.insert(
        "tags".into(),
        toml::Value::Array(vec![toml::Value::String("unknown-tag".into())]),
    );
    fs::write(&adr_path, toml::to_string_pretty(&doc)?)?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_tag_snapshot(
        "check_rejects_unknown_tag",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}

#[test]
fn test_check_accepts_registered_tag() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();

    register_tags(temp_dir.path(), &["caching"])?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "add", "ADR-0001", "tags", "caching"],
            &["check"],
        ],
    )?;
    assert_tag_snapshot(
        "check_accepts_registered_tag",
        normalize_output(&output, temp_dir.path(), &date)?,
    );
    Ok(())
}
