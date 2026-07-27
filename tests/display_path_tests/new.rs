use super::*;

#[test]
fn test_rfc_new_dry_run_display_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let output = run_commands(temp_dir.path(), &[&["rfc", "new", "New RFC", "--dry-run"]])?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_new_dry_run_display_path() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let output = run_commands(
        temp_dir.path(),
        &[&["work", "new", "New Work", "--dry-run"]],
    )?;
    assert_display_path_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_work_new_dry_run_selects_same_suffix_as_execution() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    run_commands(temp_dir.path(), &[&["work", "new", "Repeated title"]])?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "new", "Repeated title", "--dry-run"]],
    )?;

    assert!(
        output.contains(&format!("{date}-repeated-title-001.toml")),
        "{output}"
    );
    assert!(
        !temp_dir
            .path()
            .join(format!("gov/work/{date}-repeated-title-001.toml"))
            .exists()
    );
    Ok(())
}
