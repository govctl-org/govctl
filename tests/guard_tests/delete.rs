use super::*;

#[test]
fn test_guard_delete_unreferenced() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-TEMP", "true")?;

    let guard_path = temp_dir.path().join("gov/guard/guard-temp.toml");
    assert!(guard_path.exists());

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-TEMP"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(!guard_path.exists(), "guard file should be deleted");
    Ok(())
}

#[test]
fn test_guard_delete_blocked_by_config() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-IMPORTANT", "true")?;
    append_verification_config(temp_dir.path(), true, &["GUARD-IMPORTANT"])?;

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-IMPORTANT"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "should block delete: {}",
        output
    );
    assert!(
        output.contains("default_guards"),
        "should mention config: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_delete_blocked_by_work_item() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-REQUIRED", "true")?;
    write_guarded_work_item(temp_dir.path(), "WI-2026-01-01-001", "GUARD-REQUIRED", None)?;

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-REQUIRED"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "should block delete: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_delete_blocked_by_work_item_waiver() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-WAIVED", "true")?;
    write_guarded_work_item(
        temp_dir.path(),
        "WI-2026-01-01-001",
        "GUARD-WAIVED",
        Some("covered elsewhere"),
    )?;

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-WAIVED"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("Waiver in work item WI-2026-01-01-001"),
        "should mention waiver blocker: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_delete_force_does_not_bypass_reference_checks() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-FORCED", "true")?;
    append_verification_config(temp_dir.path(), true, &["GUARD-FORCED"])?;

    let output = run_commands(
        temp_dir.path(),
        &[&["guard", "delete", "GUARD-FORCED", "--force"]],
    )?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("still referenced"),
        "force should not bypass reference checks: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_delete_missing_returns_coded_error() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "delete", "GUARD-MISSING"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1002]"), "output: {}", output);
    Ok(())
}
