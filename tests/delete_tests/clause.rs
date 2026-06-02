use super::*;

fn write_rfc_fixture(
    project_dir: &std::path::Path,
    title: &str,
    version: &str,
    status: &str,
    phase: &str,
    clauses: &[(&str, &str, &str)],
    changelog_entry: &str,
) -> std::io::Result<std::path::PathBuf> {
    let rfc_dir = project_dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    let clause_paths = clauses
        .iter()
        .map(|(id, _, _)| format!("\"clauses/{id}.toml\""))
        .collect::<Vec<_>>()
        .join(", ");

    fs::write(
        rfc_dir.join("rfc.toml"),
        format!(
            r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "{title}"
version = "{version}"
status = "{status}"
phase = "{phase}"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Specification"
clauses = [{clause_paths}]

[[changelog]]
version = "{version}"
date = "2026-01-01"
{changelog_entry}
"#
        ),
    )?;

    for (id, title, text) in clauses {
        write_clause_fixture(&rfc_dir, id, title, version, text)?;
    }

    Ok(rfc_dir)
}

fn write_clause_fixture(
    rfc_dir: &std::path::Path,
    id: &str,
    title: &str,
    since: &str,
    text: &str,
) -> std::io::Result<()> {
    fs::write(
        rfc_dir.join(format!("clauses/{id}.toml")),
        format!(
            r#"#:schema ../../schema/clause.schema.json

[govctl]
schema = 1
id = "{id}"
title = "{title}"
kind = "normative"
status = "active"
since = "{since}"

[content]
text = "{text}"
"#
        ),
    )
}

#[test]
fn test_delete_clause_safeguard_normative() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_fixture(
        temp_dir.path(),
        "Normative RFC",
        "1.0.0",
        "normative",
        "stable",
        &[(
            "C-LOCKED",
            "Locked Clause",
            "This clause cannot be deleted - RFC is normative.",
        )],
        r#"added = ["Initial release"]"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "delete", "RFC-0001:C-LOCKED", "-f"]],
    )?;
    assert_delete_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

#[test]
fn test_delete_clause_success_draft() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_fixture(
        temp_dir.path(),
        "Draft RFC",
        "0.1.0",
        "draft",
        "spec",
        &[
            ("C-KEEP", "Keep This Clause", "This clause will remain."),
            (
                "C-DELETE",
                "Delete This Clause",
                "This clause will be deleted.",
            ),
        ],
        r#"notes = "Initial draft""#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["clause", "delete", "RFC-0001:C-DELETE", "-f"],
            &["clause", "list", "RFC-0001"],
            &["check"],
        ],
    )?;
    assert_delete_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

#[test]
fn test_delete_clause_safeguard_referenced_by_artifacts() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let rfc_dir = write_rfc_fixture(
        temp_dir.path(),
        "Draft RFC",
        "0.1.0",
        "draft",
        "spec",
        &[(
            "C-DELETE",
            "Referenced Clause",
            "This clause is still referenced.",
        )],
        r#"notes = "Initial draft""#,
    )?;

    let commands: Vec<Vec<String>> = vec![
        vec![
            "rfc".to_string(),
            "new".to_string(),
            "Referencing RFC".to_string(),
        ],
        vec![
            "rfc".to_string(),
            "add".to_string(),
            "RFC-0002".to_string(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "adr".to_string(),
            "new".to_string(),
            "Referencing ADR".to_string(),
        ],
        vec![
            "adr".to_string(),
            "add".to_string(),
            "ADR-0001".to_string(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Referencing work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            work_id.clone(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "guard".to_string(),
            "new".to_string(),
            "Clause guard".to_string(),
        ],
        vec![
            "guard".to_string(),
            "add".to_string(),
            "GUARD-CLAUSE-GUARD".to_string(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "clause".to_string(),
            "delete".to_string(),
            "RFC-0001:C-DELETE".to_string(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    assert!(output.contains("exit: 1"), "output: {output}");
    assert!(output.contains("Cannot delete clause"), "output: {output}");
    assert!(output.contains("RFC-0002"), "output: {output}");
    assert!(output.contains("ADR-0001"), "output: {output}");
    assert!(output.contains(&work_id), "output: {output}");
    assert!(output.contains("GUARD-CLAUSE-GUARD"), "output: {output}");
    assert!(
        rfc_dir.join("clauses/C-DELETE.toml").exists(),
        "referenced clause should remain on disk"
    );

    Ok(())
}
