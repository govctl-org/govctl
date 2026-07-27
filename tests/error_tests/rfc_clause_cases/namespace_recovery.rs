use super::*;

fn create_rfc_and_clause(temp_dir: &std::path::Path) -> common::TestResult {
    run_commands(
        temp_dir,
        &[
            &["rfc", "new", "Namespace recovery", "--id", "RFC-0001"],
            &["clause", "new", "RFC-0001:C-ONE", "Original clause title"],
        ],
    )?;
    Ok(())
}

#[test]
fn test_rfc_shared_verbs_reject_clause_ids_with_canonical_guidance() -> common::TestResult {
    let temp_dir = init_project()?;
    create_rfc_and_clause(temp_dir.path())?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "get", "RFC-0001:C-ONE", "title"],
            &["rfc", "show", "RFC-0001:C-ONE", "--history"],
            &[
                "rfc",
                "edit",
                "RFC-0001:C-ONE",
                "title",
                "--set",
                "Changed title",
            ],
            &["clause", "get", "RFC-0001:C-ONE", "title"],
        ],
    )?;

    for suggestion in [
        "govctl clause get RFC-0001:C-ONE title",
        "govctl clause show RFC-0001:C-ONE --history",
        "govctl clause edit RFC-0001:C-ONE title --set \"Changed title\"",
    ] {
        assert!(output.contains(suggestion), "{output}");
    }
    assert_eq!(output.matches("error[E0821]").count(), 3, "{output}");
    assert!(output.contains("Original clause title"), "{output}");
    assert!(!output.contains("Set RFC-0001:C-ONE.title"), "{output}");
    Ok(())
}

#[test]
fn test_nested_rfc_clause_form_points_to_root_clause_namespace() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "clause", "show", "RFC-0001:C-ONE"]],
    )?;

    assert!(output.contains("error[E0821]"), "{output}");
    assert!(
        output.contains("govctl clause show RFC-0001:C-ONE"),
        "{output}"
    );
    assert!(output.contains("exit: 1"), "{output}");
    Ok(())
}

#[test]
fn test_rfc_clause_owned_path_gives_namespace_only_guidance() -> common::TestResult {
    let temp_dir = init_project()?;
    create_rfc_and_clause(temp_dir.path())?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "edit",
            "RFC-0001",
            "clauses[0].text",
            "--set",
            "Changed",
        ]],
    )?;

    assert!(output.contains("error[E0821]"), "{output}");
    assert!(
        output.contains("govctl clause edit <RFC-ID:C-NAME> ..."),
        "{output}"
    );
    assert!(
        output.contains("the rejected path does not identify a Clause"),
        "{output}"
    );
    assert!(!output.contains("RFC-0001:C-ONE"), "{output}");
    Ok(())
}

#[test]
fn test_nonshared_rfc_verb_does_not_invent_clause_command() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "render", "RFC-0001:C-ONE", "--dry-run"]],
    )?;

    assert!(output.contains("error[E0821]"), "{output}");
    assert!(
        output.contains("Use the `govctl clause` namespace"),
        "{output}"
    );
    assert!(!output.contains("govctl clause render"), "{output}");
    Ok(())
}

#[test]
fn test_malformed_clause_like_id_does_not_trigger_namespace_recovery() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(temp_dir.path(), &[&["rfc", "get", "RFC-X:C-", "title"]])?;

    assert!(!output.contains("error[E0821]"), "{output}");
    assert!(!output.contains("identifies a Clause"), "{output}");
    assert!(output.contains("error[E0202]"), "{output}");
    Ok(())
}

#[test]
fn test_rfc_new_rejects_clause_reference_in_explicit_id_position() -> common::TestResult {
    let temp_dir = init_project()?;
    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "new", "Misrouted Clause", "--id", "RFC-0001:C-ONE"]],
    )?;

    assert!(output.contains("error[E0821]"), "{output}");
    assert!(
        output.contains("identifies a Clause, not an RFC"),
        "{output}"
    );
    assert!(
        output.contains("Use the `govctl clause` namespace"),
        "{output}"
    );
    assert!(
        !temp_dir.path().join("gov/rfc/RFC-0001:C-ONE").exists(),
        "rejected RFC creation must not create storage"
    );
    Ok(())
}

#[test]
fn test_rfc_new_rejects_path_traversing_explicit_id_without_mutation() -> common::TestResult {
    let temp_dir = tempfile::TempDir::new()?;
    let project_dir = temp_dir.path().join("project");
    fs::create_dir(&project_dir)?;
    run_commands(&project_dir, &[&["init"]])?;
    let escaped_dir = temp_dir.path().join("escaped-rfc");

    let output = run_commands(
        &project_dir,
        &[&[
            "rfc",
            "new",
            "Escaped RFC",
            "--id",
            "RFC-X/../../../../escaped-rfc",
        ]],
    )?;

    assert!(output.contains("error[E0110]"), "{output}");
    assert!(output.contains("expected RFC-NNNN"), "{output}");
    assert!(!escaped_dir.exists(), "rejected RFC ID escaped storage");
    assert!(
        fs::read_dir(project_dir.join("gov/rfc"))?.next().is_none(),
        "rejected RFC creation must not create storage"
    );
    Ok(())
}

#[test]
fn test_rfc_new_rejects_exhausted_automatic_id_namespace() -> common::TestResult {
    let temp_dir = init_project()?;
    fs::create_dir(temp_dir.path().join("gov/rfc/RFC-9999"))?;

    let output = run_commands(temp_dir.path(), &[&["rfc", "new", "Overflow RFC"]])?;

    assert!(output.contains("error[E0110]"), "{output}");
    assert!(output.contains("namespace exhausted"), "{output}");
    assert!(
        !temp_dir.path().join("gov/rfc/RFC-10000").exists(),
        "exhausted RFC namespace must not create invalid storage"
    );
    Ok(())
}

#[test]
fn test_rfc_new_dry_run_rejects_existing_explicit_id() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "new", "Existing RFC", "--id", "RFC-0001"]],
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "new",
            "Replacement RFC",
            "--id",
            "RFC-0001",
            "--dry-run",
        ]],
    )?;

    assert!(output.contains("error[E0109]"), "{output}");
    assert!(!output.contains("Would write"), "{output}");
    Ok(())
}

#[test]
fn test_rfc_supersede_rejects_clause_reference_as_replacement() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "new", "Source RFC", "--id", "RFC-0001"]],
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&[
            "rfc",
            "supersede",
            "RFC-0001",
            "--by",
            "RFC-0002:C-ONE",
            "--force",
        ]],
    )?;

    assert!(output.contains("error[E0821]"), "{output}");
    assert!(
        output.contains("identifies a Clause, not an RFC"),
        "{output}"
    );
    assert!(
        output.contains("Use the `govctl clause` namespace"),
        "{output}"
    );
    let status = run_commands(temp_dir.path(), &[&["rfc", "get", "RFC-0001", "status"]])?;
    assert!(status.contains("draft"), "{status}");
    Ok(())
}
