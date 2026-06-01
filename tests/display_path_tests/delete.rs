use super::*;

#[test]
fn test_delete_work_dry_run_display_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir)?;

    let work_filename = format!("{}-test-work.toml", date);
    fs::write(
        work_dir.join(&work_filename),
        format!(
            r#"[govctl]
schema = 1
id = "WI-{}-001"
title = "Test Work to Delete"
status = "queue"
created = "{}"
refs = []

[content]
description = "Test description"
acceptance_criteria = []
notes = []
"#,
            date, date
        ),
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "delete", &format!("WI-{}-001", date), "--dry-run"]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_delete_clause_dry_run_display_path() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
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
clauses = ["clauses/C-TO-DELETE.toml"]

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "Initial draft"
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-TO-DELETE.toml"),
        r#"#:schema ../../schema/clause.schema.json

[govctl]
schema = 1
id = "C-TO-DELETE"
title = "Clause To Delete"
kind = "normative"
status = "active"

[content]
text = "This clause will be deleted."
"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "delete", "RFC-0001:C-TO-DELETE", "--dry-run"]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
