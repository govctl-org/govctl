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
fn test_delete_clause_success_current_normative_spec_candidate() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_fixture(
        temp_dir.path(),
        "Normative spec RFC",
        "1.1.0",
        "normative",
        "spec",
        &[
            ("C-KEEP", "Keep This Clause", "This clause will remain."),
            (
                "C-DELETE",
                "Delete This Clause",
                "This candidate Clause references [[RFC-0001:C-DELETE]] itself.",
            ),
        ],
        r#"added = ["Current candidate"]"#,
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
fn test_delete_clause_rejects_inherited_normative_spec_clause() -> TestResult {
    let (temp_dir, _) = init_project_with_date()?;
    let rfc_dir = write_rfc_fixture(
        temp_dir.path(),
        "Normative spec RFC",
        "1.1.0",
        "normative",
        "spec",
        &[(
            "C-INHERITED",
            "Inherited Clause",
            "This clause was introduced in an earlier version.",
        )],
        r#"added = ["Current candidate"]"#,
    )?;
    let clause_path = rfc_dir.join("clauses/C-INHERITED.toml");
    let mut clause: toml::Value = toml::from_str(&fs::read_to_string(&clause_path)?)?;
    clause["govctl"]["since"] = toml::Value::String("1.0.0".to_string());
    fs::write(&clause_path, toml::to_string_pretty(&clause)?)?;
    let rfc_before = fs::read(rfc_dir.join("rfc.toml"))?;
    let clause_before = fs::read(&clause_path)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "delete", "RFC-0001:C-INHERITED", "-f"]],
    )?;

    assert!(output.contains("error[E0104]"), "output: {output}");
    assert!(output.contains("version=1.1.0"), "output: {output}");
    assert!(output.contains("clause since=1.0.0"), "output: {output}");
    assert_eq!(fs::read(rfc_dir.join("rfc.toml"))?, rfc_before);
    assert_eq!(fs::read(&clause_path)?, clause_before);
    Ok(())
}

#[test]
fn test_delete_clause_safeguard_referenced_by_artifacts() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let rfc_dir = write_rfc_fixture(
        temp_dir.path(),
        "Normative spec RFC",
        "0.1.0",
        "normative",
        "spec",
        &[(
            "C-DELETE",
            "Referenced Clause",
            "This clause is still referenced.",
        )],
        r#"added = ["Current candidate"]"#,
    )?;

    let commands: Vec<Vec<String>> = vec![
        command(&["rfc", "new", "Referencing RFC"]),
        command(&["rfc", "add", "RFC-0002", "refs", "RFC-0001:C-DELETE"]),
        command(&["adr", "new", "Referencing ADR"]),
        command(&["adr", "add", "ADR-0001", "refs", "RFC-0001:C-DELETE"]),
        work_new("Referencing work item"),
        work_add_field(&work_id, "refs", "RFC-0001:C-DELETE"),
        command(&["guard", "new", "Clause guard"]),
        command(&[
            "guard",
            "add",
            "GUARD-CLAUSE-GUARD",
            "refs",
            "RFC-0001:C-DELETE",
        ]),
        command(&["clause", "delete", "RFC-0001:C-DELETE", "-f"]),
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

#[test]
fn test_delete_clause_reports_inline_and_supersession_referrers() -> TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let work_id = first_work_id(&date);

    let rfc_dir = write_rfc_fixture(
        temp_dir.path(),
        "Normative spec RFC",
        "0.1.0",
        "normative",
        "spec",
        &[(
            "C-DELETE",
            "Referenced Clause",
            "This clause is still referenced.",
        )],
        r#"added = ["Current candidate"]"#,
    )?;

    let commands: Vec<Vec<String>> = vec![
        command(&[
            "clause",
            "new",
            "RFC-0001:C-OLD",
            "Old Clause",
            "-s",
            "Specification",
            "-k",
            "normative",
        ]),
        command(&[
            "clause",
            "supersede",
            "RFC-0001:C-OLD",
            "--by",
            "C-DELETE",
            "--force",
        ]),
        command(&["rfc", "new", "Inline RFC"]),
        command(&[
            "clause",
            "new",
            "RFC-0002:C-INLINE",
            "Inline Clause",
            "-s",
            "Specification",
            "-k",
            "normative",
        ]),
        command(&[
            "clause",
            "edit",
            "RFC-0002:C-INLINE",
            "text",
            "--set",
            "Uses [[RFC-0001:C-DELETE]].",
        ]),
        command(&["adr", "new", "Inline ADR"]),
        command(&[
            "adr",
            "edit",
            "ADR-0001",
            "content.context",
            "--set",
            "Uses [[RFC-0001:C-DELETE]].",
        ]),
        work_new("Inline work item"),
        command(&[
            "work",
            "edit",
            &work_id,
            "description",
            "--set",
            "Uses [[RFC-0001:C-DELETE]].",
        ]),
        command(&["clause", "delete", "RFC-0001:C-DELETE", "-f"]),
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    assert!(output.contains("error[E0211]"), "output: {output}");
    assert!(output.contains("RFC-0001:C-OLD"), "output: {output}");
    assert!(output.contains("RFC-0002:C-INLINE"), "output: {output}");
    assert!(output.contains("ADR-0001"), "output: {output}");
    assert!(output.contains(&work_id), "output: {output}");
    assert!(
        rfc_dir.join("clauses/C-DELETE.toml").exists(),
        "referenced clause should remain on disk"
    );
    Ok(())
}
