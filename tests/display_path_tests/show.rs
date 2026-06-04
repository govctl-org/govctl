use super::*;

#[test]
fn test_show_rfc_missing_returns_scope_context() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "show", "RFC-9999"]])?;

    assert_show_missing_scope(
        &output,
        temp_dir.path(),
        "error[E0102]: RFC not found: RFC-9999",
        "gov/rfc",
    );
    Ok(())
}

#[test]
fn test_show_adr_missing_returns_scope_context() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["adr", "show", "ADR-9999"]])?;

    assert_show_missing_scope(
        &output,
        temp_dir.path(),
        "error[E0302]: ADR not found: ADR-9999",
        "gov/adr",
    );
    Ok(())
}

#[test]
fn test_show_work_missing_returns_scope_context() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["work", "show", "WI-9999-01-01-001"]])?;

    assert_show_missing_scope(
        &output,
        temp_dir.path(),
        "error[E0402]: Work item not found: WI-9999-01-01-001",
        "gov/work",
    );
    Ok(())
}

#[test]
fn test_show_clause_missing_returns_scope_context() -> common::TestResult {
    let temp_dir = init_project()?;
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
clauses = []

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "Initial draft"
"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "show", "RFC-0001:C-MISSING"]],
    )?;

    assert_show_missing_scope(
        &output,
        temp_dir.path(),
        "error[E0202]: Clause not found: RFC-0001:C-MISSING",
        "gov/rfc/RFC-0001/clauses",
    );
    Ok(())
}
