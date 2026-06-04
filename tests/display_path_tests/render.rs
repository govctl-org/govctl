use super::*;

#[test]
fn test_render_rfc_display_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
id = "RFC-0001"
title = "Test RFC"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Specification"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "Initial draft"
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-TEST.toml"),
        r#"[govctl]
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"

[content]
text = "Test clause content."
"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "render", "RFC-0001", "--dry-run"]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_render_adr_display_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let adr_dir = temp_dir.path().join("gov/adr");
    fs::create_dir_all(&adr_dir)?;

    fs::write(
        adr_dir.join("ADR-0001-test-decision.toml"),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Test Decision"
status = "proposed"
date = "2026-01-01"
refs = []

[content]
context = "Test context"
decision = "Test decision"
alternatives = []
consequences = "Test consequences"
"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["adr", "render", "ADR-0001", "--dry-run"]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_render_work_display_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir)?;

    let work_filename = format!("{}-test-work.toml", date);
    fs::write(
        work_dir.join(&work_filename),
        format!(
            r#"[govctl]
schema = 1
id = "{}"
title = "Test Work"
status = "active"
created = "{}"
started = "{}"
refs = []

[content]
description = "Test description"
acceptance_criteria = []
notes = []
"#,
            work_id, date, date
        ),
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "render", &work_id, "--dry-run"]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_render_rfc_missing_returns_scope_context() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "render", "RFC-9999"]])?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_render_adr_missing_returns_scope_context() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["adr", "render", "ADR-9999"]])?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_render_work_missing_returns_scope_context() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(temp_dir.path(), &[&["work", "render", "WI-9999-01-01-001"]])?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
