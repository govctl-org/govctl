use super::*;

fn write_draft_rfc_fixture(project_dir: &std::path::Path) -> common::TestResult {
    let rfc_dir = project_dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;
    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "Draft RFC"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Specification"
clauses = []

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "Initial draft"
"#,
    )?;
    Ok(())
}

#[test]
fn test_rfc_set_dry_run_display_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_draft_rfc_fixture(temp_dir.path())?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "set",
            "RFC-0001",
            "title",
            "Updated Title",
            "--dry-run",
        ]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_bump_dry_run_display_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_draft_rfc_fixture(temp_dir.path())?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "bump",
            "RFC-0001",
            "--change",
            "fix: test change",
            "--dry-run",
        ]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
