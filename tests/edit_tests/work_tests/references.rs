use super::*;

#[test]
fn test_work_add_ref() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "refs",
                "RFC-0001",
            ],
            &["work", "get", &format!("WI-{}-001", date), "refs"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_depends_on_add_get_show_remove() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let dependency_id = format!("WI-{date}-001");
    let dependent_id = format!("WI-{date}-002");

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            vec![
                "work".to_string(),
                "new".to_string(),
                "Dependency Task".to_string(),
            ],
            vec![
                "work".to_string(),
                "new".to_string(),
                "Dependent Task".to_string(),
            ],
            vec![
                "work".to_string(),
                "add".to_string(),
                dependent_id.clone(),
                "refs".to_string(),
                "RFC-0001".to_string(),
            ],
            vec![
                "work".to_string(),
                "add".to_string(),
                dependent_id.clone(),
                "depends_on".to_string(),
                dependency_id.clone(),
            ],
            vec![
                "work".to_string(),
                "get".to_string(),
                dependent_id.clone(),
                "depends_on".to_string(),
            ],
            vec!["work".to_string(), "show".to_string(), dependent_id.clone()],
            vec![
                "work".to_string(),
                "remove".to_string(),
                dependent_id.clone(),
                "depends_on".to_string(),
                dependency_id.clone(),
            ],
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
    let first_id = format!("WI-{date}-001");
    let second_id = format!("WI-{date}-002");
    let third_id = format!("WI-{date}-003");
    let unknown_id = format!("WI-{date}-999");

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            vec!["work".to_string(), "new".to_string(), "First".to_string()],
            vec!["work".to_string(), "new".to_string(), "Second".to_string()],
            vec!["work".to_string(), "new".to_string(), "Third".to_string()],
            vec![
                "work".to_string(),
                "add".to_string(),
                second_id.clone(),
                "depends_on".to_string(),
                "RFC-0001".to_string(),
            ],
            vec![
                "work".to_string(),
                "add".to_string(),
                second_id.clone(),
                "depends_on".to_string(),
                unknown_id.clone(),
            ],
            vec![
                "work".to_string(),
                "add".to_string(),
                second_id.clone(),
                "depends_on".to_string(),
                first_id.clone(),
            ],
            vec![
                "work".to_string(),
                "add".to_string(),
                first_id.clone(),
                "depends_on".to_string(),
                second_id.clone(),
            ],
            vec![
                "work".to_string(),
                "add".to_string(),
                first_id.clone(),
                "depends_on".to_string(),
                third_id.clone(),
            ],
            vec![
                "work".to_string(),
                "edit".to_string(),
                first_id.clone(),
                "depends_on[0]".to_string(),
                "--set".to_string(),
                second_id.clone(),
            ],
            vec![
                "work".to_string(),
                "get".to_string(),
                first_id.clone(),
                "depends_on".to_string(),
            ],
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
