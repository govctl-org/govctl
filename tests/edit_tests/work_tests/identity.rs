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
    assert!(std::fs::read_to_string(first_path)?.contains(&format!("id = \"WI-{date}-001\"")));
    assert!(std::fs::read_to_string(second_path)?.contains(&format!("id = \"WI-{date}-002\"")));
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
    let first = id_pattern.captures(&ids[0]).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unexpected work item id: {}", ids[0]),
        )
    })?;
    let second = id_pattern.captures(&ids[1]).ok_or_else(|| {
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
