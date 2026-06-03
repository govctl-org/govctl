use super::*;

const REFS: &str = "refs";
const DEPENDS_ON: &str = "depends_on";

fn work_add_ref(id: &str, target: &str) -> Vec<String> {
    work_add_field(id, REFS, target)
}

fn work_get_dependencies(id: &str) -> Vec<String> {
    work_get_field(id, DEPENDS_ON)
}

fn work_edit_dependency_index(id: &str, index: usize, dependency: &str) -> Vec<String> {
    let field = format!("{DEPENDS_ON}[{index}]");
    command(&["work", "edit", id, &field, "--set", dependency])
}

fn work_edit_ref_index(id: &str, index: usize, target: &str) -> Vec<String> {
    let field = format!("{REFS}[{index}]");
    command(&["work", "edit", id, &field, "--set", target])
}

#[test]
fn test_work_add_ref() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = work_id(&date, 1);
    common::write_minimal_rfc(temp_dir.path(), "RFC-0001", "Referenced RFC")?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Test Task"),
            work_add_ref(&id, "RFC-0001"),
            work_get_field(&id, REFS),
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_refs_validate_unknown_targets_and_accept_all_artifact_types() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_id = work_id(&date, 1);
    let second_id = work_id(&date, 2);

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            command(&["rfc", "new", "Governing RFC"]),
            command(&["adr", "new", "Test Decision"]),
            work_new("First Task"),
            work_new("Second Task"),
            work_add_ref(&first_id, "RFC-0001"),
            work_add_ref(&first_id, "ADR-0001"),
            work_add_ref(&first_id, &second_id),
            work_add_ref(&first_id, "WI-2026-01-01-999"),
            work_edit_ref_index(&first_id, 0, "WI-2026-01-01-999"),
            work_get_field(&first_id, REFS),
        ],
    )?;

    assert!(output.contains("error[E0404]"), "output: {}", output);
    assert!(
        output.contains(&format!("Added '{second_id}' to {first_id}.refs")),
        "output: {}",
        output
    );
    assert!(
        !output.contains("Added 'WI-2026-01-01-999'"),
        "output: {}",
        output
    );
    assert!(
        !output.contains("Set "),
        "invalid indexed refs set should not be reported as successful: {}",
        output
    );
    assert!(
        output.contains(&format!(
            "$ govctl work get {first_id} refs\nRFC-0001, ADR-0001, {second_id}"
        )),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_depends_on_add_get_show_remove() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let dependency_id = work_id(&date, 1);
    let dependent_id = work_id(&date, 2);
    common::write_minimal_rfc(temp_dir.path(), "RFC-0001", "Referenced RFC")?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Dependency Task"),
            work_new("Dependent Task"),
            work_add_ref(&dependent_id, "RFC-0001"),
            work_add_dependency(&dependent_id, &dependency_id),
            work_get_dependencies(&dependent_id),
            work_show(&dependent_id),
            work_remove_dependency(&dependent_id, &dependency_id),
        ],
    )?;

    assert!(
        output.contains(&format!(
            "Added '{dependency_id}' to {dependent_id}.depends_on"
        )),
        "output: {}",
        output
    );
    assert!(
        output.contains(&format!(
            "$ govctl work get {dependent_id} depends_on\n{dependency_id}"
        )),
        "output: {}",
        output
    );
    assert!(output.contains("**References:**"), "output: {}", output);
    assert!(output.contains("**Depends On:**"), "output: {}", output);
    assert!(
        output.contains(&format!(
            "Removed '{dependency_id}' from {dependent_id}.depends_on"
        )),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_depends_on_rejects_invalid_unknown_and_cycle() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let first_id = work_id(&date, 1);
    let second_id = work_id(&date, 2);
    let third_id = work_id(&date, 3);
    let unknown_id = format!("WI-{date}-999");

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("First"),
            work_new("Second"),
            work_new("Third"),
            work_add_dependency(&second_id, "RFC-0001"),
            work_add_dependency(&second_id, &unknown_id),
            work_add_dependency(&second_id, &first_id),
            work_add_dependency(&first_id, &second_id),
            work_add_dependency(&first_id, &third_id),
            work_edit_dependency_index(&first_id, 0, &second_id),
            work_get_dependencies(&first_id),
        ],
    )?;

    assert!(output.contains("error[E0409]"), "output: {}", output);
    assert!(
        output.contains("must be a work item ID"),
        "output: {}",
        output
    );
    assert!(output.contains("error[E0410]"), "output: {}", output);
    assert!(
        output.contains("unknown work item dependency"),
        "output: {}",
        output
    );
    assert!(
        output.contains(&format!("Added '{first_id}' to {second_id}.depends_on")),
        "output: {}",
        output
    );
    assert!(output.contains("error[E0411]"), "output: {}", output);
    assert!(
        output.contains("cyclic work item dependency"),
        "output: {}",
        output
    );
    assert!(
        output.contains(&format!("Added '{third_id}' to {first_id}.depends_on")),
        "output: {}",
        output
    );
    assert!(
        output.contains(&format!(
            "$ govctl work get {first_id} depends_on\n{third_id}"
        )),
        "output: {}",
        output
    );
    Ok(())
}
