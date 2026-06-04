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
