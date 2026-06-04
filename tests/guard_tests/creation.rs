use super::*;

#[test]
fn test_guard_new_scaffolds_file() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "clippy lint"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-CLIPPY-LINT"), "output: {}", output);
    assert!(
        output.contains("verification.default_guards"),
        "should hint about default_guards: {}",
        output
    );

    let guard_path = temp_dir.path().join("gov/guard/clippy-lint.toml");
    assert!(guard_path.exists(), "guard file should be created");

    let content = fs::read_to_string(&guard_path)?;
    assert!(content.contains("GUARD-CLIPPY-LINT"));
    assert!(content.contains("clippy lint"));
    assert!(content.contains("[check]"));
    Ok(())
}

#[test]
fn test_guard_new_duplicate_rejected() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "true")?;

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "echo"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("already exists"),
        "should reject duplicate: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_new_invalid_title_rejected_with_code() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "new", "123 !!!"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1006]"), "output: {}", output);
    Ok(())
}
