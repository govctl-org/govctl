use super::*;

fn rfc_dir(project_dir: &std::path::Path) -> std::path::PathBuf {
    project_dir.join("gov/rfc/RFC-0001")
}

fn write_rfc_toml(project_dir: &std::path::Path, content: &str) -> common::TestResult {
    let rfc_dir = rfc_dir(project_dir);
    fs::create_dir_all(rfc_dir.join("clauses"))?;
    fs::write(rfc_dir.join("rfc.toml"), content)?;
    Ok(())
}

fn write_clause_toml(
    project_dir: &std::path::Path,
    file_name: &str,
    content: &str,
) -> common::TestResult {
    let rfc_dir = rfc_dir(project_dir);
    fs::create_dir_all(rfc_dir.join("clauses"))?;
    fs::write(rfc_dir.join("clauses").join(file_name), content)?;
    Ok(())
}

#[test]
fn test_broken_superseded_check() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Broken Superseded Test"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Clauses"
clauses = ["clauses/C-OLD.toml", "clauses/C-NEW.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-OLD.toml",
        r#"[govctl]
schema = 1
id = "C-OLD"
title = "Old Clause"
kind = "normative"
status = "superseded"
superseded_by = "C-NONEXISTENT"
since = "1.0.0"

[content]
text = "This clause is superseded."
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-NEW.toml",
        r#"[govctl]
schema = 1
id = "C-NEW"
title = "New Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This is the new clause."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

/// Test: RFC has invalid status/phase combination (draft + stable)
#[test]
fn test_invalid_transition_check() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Invalid Transition Test"
version = "0.1.0"
status = "draft"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
added = ["Initial draft"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "0.1.0"

[content]
text = "A test clause in an invalid RFC."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
