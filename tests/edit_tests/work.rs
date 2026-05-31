// ============================================================================
// Work Item Field Edit Tests
// ============================================================================

fn read_work_ids(
    project_dir: &std::path::Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(project_dir.join("gov").join("work"))? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let id = content
            .lines()
            .find_map(|line| line.strip_prefix("id = \"")?.strip_suffix('"'))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("missing work item id in {}", path.display()),
                )
            })?;
        ids.push(id.to_string());
    }
    ids.sort();
    Ok(ids)
}

fn set_work_item_id_strategy(project_dir: &std::path::Path, strategy: &str) -> common::TestResult {
    let config_path = project_dir.join("gov").join("config.toml");
    let mut content = std::fs::read_to_string(&config_path)?;
    content.push_str(&format!("\n[work_item]\nid_strategy = \"{strategy}\"\n"));
    std::fs::write(config_path, content)?;
    Ok(())
}

#[test]
fn test_work_get_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "get", &format!("WI-{}-001", date), "title"],
            &["work", "get", &format!("WI-{}-001", date), "status"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_set_title() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Original Title"],
            &[
                "work",
                "set",
                &format!("WI-{}-001", date),
                "title",
                "New Title",
            ],
            &["work", "list", "all"],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_set_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "set", &work_id, "status", "active"],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work move"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_set_acceptance_criteria_status_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &work_id,
                "acceptance_criteria",
                "add: Test criterion",
            ],
            &[
                "work",
                "set",
                &work_id,
                "acceptance_criteria[0].status",
                "done",
            ],
        ],
    )?;
    assert!(output.contains("error[E0804]"), "output: {}", output);
    assert!(output.contains("govctl work tick"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_add_acceptance_criteria() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 2",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_add_and_edit_acceptance_criteria_extras() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let work_id = format!("WI-{}-001", date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Category Extras"],
            &[
                "work",
                "add",
                &work_id,
                "acceptance_criteria",
                "Add without prefix",
                "--category",
                "fixed",
                "--scope",
                "legacy-add",
            ],
            &[
                "work",
                "edit",
                &work_id,
                "acceptance_criteria",
                "--add",
                "Edit without prefix",
                "--category",
                "changed",
                "--scope",
                "legacy-edit",
            ],
            &["work", "show", &work_id],
        ],
    )?;

    assert!(
        output.contains("- ○ fixed: Add without prefix"),
        "output: {}",
        output
    );
    assert!(
        output.contains("- ○ changed: Edit without prefix"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_tick_acceptance_criteria() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 2",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "Criterion 1",
                "-s",
                "done",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_edit_tick_indexed_path_canonical() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let wi_id = format!("WI-{}-001", date);

    let commands = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Canonical Tick".to_string(),
        ],
        vec![
            "work".to_string(),
            "edit".to_string(),
            wi_id.clone(),
            "content.acceptance_criteria".to_string(),
            "--add".to_string(),
            "add: Criterion 1".to_string(),
        ],
        vec![
            "work".to_string(),
            "edit".to_string(),
            wi_id.clone(),
            "content.acceptance_criteria[0]".to_string(),
            "--tick".to_string(),
            "done".to_string(),
        ],
        vec!["work".to_string(), "show".to_string(), wi_id],
    ];

    let output = common::run_dynamic_commands(temp_dir.path(), &commands)?;

    assert!(
        output.contains("Added 'add: Criterion 1' to WI-"),
        "output: {}",
        output
    );
    assert!(
        output.contains("Marked 'Criterion 1' as done"),
        "output: {}",
        output
    );
    assert!(
        output.contains("- ✓ added: Criterion 1"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_tick_cancel_acceptance_criteria() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: Criterion 1",
            ],
            &[
                "work",
                "tick",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "Criterion 1",
                "-s",
                "cancelled",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_journal_field_surface_is_unavailable() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let wi_id = format!("WI-{date}-001");
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &["work", "add", &wi_id, "journal", "First progress update"],
            &["work", "get", &wi_id, "journal"],
            &[
                "work",
                "edit",
                &wi_id,
                "journal",
                "--add",
                "Second progress update",
            ],
            &[
                "work",
                "edit",
                &wi_id,
                "journal[0].content",
                "--set",
                "Changed history",
            ],
            &["work", "remove", &wi_id, "journal", "--all"],
            &["work", "tick", &wi_id, "journal", "--at", "0"],
            &["work", "show", &wi_id],
        ],
    )?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    assert!(
        normalized.contains("Unknown work item field: journal"),
        "output: {normalized}"
    );
    assert_eq!(
        normalized.matches("error[E0803]").count(),
        6,
        "all journal field-surface operations should be rejected: {normalized}"
    );
    assert!(
        !normalized.contains("## Journal"),
        "journal write should not persist: {normalized}"
    );
    Ok(())
}

#[test]
fn test_work_new_omits_journal_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["work", "new", "Test Task"]])?;
    assert!(output.contains("Created work item"), "output: {output}");

    let work_path = temp_dir
        .path()
        .join("gov")
        .join("work")
        .join(format!("{date}-test-task.toml"));
    let content = std::fs::read_to_string(work_path)?;

    assert!(
        !content.contains("journal") && !content.contains("[[content.journal]]"),
        "new work items should omit journal: {content}"
    );
    Ok(())
}

#[test]
fn test_work_new_same_title_uses_filename_suffix() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Duplicate Title"],
            &["work", "new", "Duplicate Title"],
        ],
    )?;
    assert_eq!(
        output.matches("Created work item").count(),
        2,
        "output: {output}"
    );

    let work_dir = temp_dir.path().join("gov").join("work");
    let first_path = work_dir.join(format!("{date}-duplicate-title.toml"));
    let second_path = work_dir.join(format!("{date}-duplicate-title-001.toml"));

    assert!(first_path.exists(), "missing {}", first_path.display());
    assert!(second_path.exists(), "missing {}", second_path.display());
    assert!(
        std::fs::read_to_string(first_path)?.contains(&format!("id = \"WI-{date}-001\""))
    );
    assert!(
        std::fs::read_to_string(second_path)?.contains(&format!("id = \"WI-{date}-002\""))
    );
    Ok(())
}

#[test]
fn test_work_new_author_hash_id_strategy_uses_shared_author_sequence() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let git_init = std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()?;
    assert!(
        git_init.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&git_init.stderr)
    );
    let git_config = std::process::Command::new("git")
        .args(["config", "user.email", "author@example.com"])
        .current_dir(temp_dir.path())
        .output()?;
    assert!(
        git_config.status.success(),
        "git config failed: {}",
        String::from_utf8_lossy(&git_config.stderr)
    );

    set_work_item_id_strategy(temp_dir.path(), "author-hash")?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Author One"],
            &["work", "new", "Author Two"],
        ],
    )?;
    assert_eq!(
        output.matches("Created work item").count(),
        2,
        "output: {output}"
    );

    let ids = read_work_ids(temp_dir.path())?;
    let id_pattern = regex::Regex::new(&format!(
        r"^WI-{}-(?P<hash>[0-9a-f]{{4}})-(?P<seq>00[12])$",
        regex::escape(&date)
    ))?;
    let first = id_pattern
        .captures(&ids[0])
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unexpected work item id: {}", ids[0]),
            )
        })?;
    let second = id_pattern
        .captures(&ids[1])
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unexpected work item id: {}", ids[1]),
            )
        })?;

    assert_eq!(&first["hash"], &second["hash"]);
    assert_eq!(&first["seq"], "001");
    assert_eq!(&second["seq"], "002");
    Ok(())
}

#[test]
fn test_work_new_random_id_strategy_uses_random_suffix() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    set_work_item_id_strategy(temp_dir.path(), "random")?;
    let output = run_commands(temp_dir.path(), &[&["work", "new", "Random ID"]])?;
    assert!(output.contains("Created work item"), "output: {output}");

    let ids = read_work_ids(temp_dir.path())?;
    assert_eq!(ids.len(), 1);
    let id_pattern = regex::Regex::new(&format!(
        r"^WI-{}-[0-9a-f]{{4}}$",
        regex::escape(&date)
    ))?;
    assert!(
        id_pattern.is_match(&ids[0]),
        "unexpected random work item id: {}",
        ids[0]
    );
    Ok(())
}

#[test]
fn test_work_add_ref() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

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
    let temp_dir = init_project()?;
    let date = today();
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
    let temp_dir = init_project()?;
    let date = today();
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

#[test]
fn test_work_remove_acceptance_criteria() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Test Task"],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: To remove",
            ],
            &[
                "work",
                "add",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "add: To keep",
            ],
            &[
                "work",
                "remove",
                &format!("WI-{}-001", date),
                "acceptance_criteria",
                "To remove",
            ],
            &["work", "show", &format!("WI-{}-001", date)],
        ],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_get_nonexistent() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "get", "WI-9999-99-999", "title"]],
    )?;
    assert_edit_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
