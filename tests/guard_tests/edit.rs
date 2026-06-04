use super::*;

#[test]
fn test_guard_set_command() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo old")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "set", "GUARD-ECHO", "command", "echo new"],
            &["guard", "get", "GUARD-ECHO", "command"],
        ],
    )?;
    assert!(output.contains("echo new"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_add_remove_refs() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "add", "GUARD-ECHO", "refs", "RFC-0001"],
            &["guard", "get", "GUARD-ECHO", "refs"],
            &["guard", "remove", "GUARD-ECHO", "refs", "RFC-0001"],
        ],
    )?;
    assert!(
        output.contains("Added 'RFC-0001' to GUARD-ECHO.refs"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl guard get GUARD-ECHO refs\nRFC-0001"),
        "output: {}",
        output
    );
    assert!(
        output.contains("Removed 'RFC-0001' from GUARD-ECHO.refs"),
        "output: {}",
        output
    );

    let guard_path = temp_dir.path().join("gov/guard/guard-echo.toml");
    let content = fs::read_to_string(guard_path)?;
    assert!(
        !content.contains("RFC-0001"),
        "ref should be removed: {}",
        content
    );
    Ok(())
}

#[test]
fn test_guard_set_timeout_secs() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "true")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "set", "GUARD-ECHO", "timeout_secs", "30"],
            &["guard", "get", "GUARD-ECHO", "timeout_secs"],
        ],
    )?;
    assert!(output.contains("30"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_nested_object_root_edit_paths() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo old")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "guard",
                "set",
                "GUARD-ECHO",
                "check.command",
                "echo nested legacy",
            ],
            &[
                "guard",
                "edit",
                "GUARD-ECHO",
                "check.timeout_secs",
                "--set",
                "45",
            ],
            &["guard", "get", "GUARD-ECHO", "command"],
            &["guard", "get", "GUARD-ECHO", "timeout_secs"],
        ],
    )?;

    assert!(
        output.contains("Set GUARD-ECHO.check.command = echo nested legacy"),
        "output: {}",
        output
    );
    assert!(
        output.contains("Set GUARD-ECHO.check.timeout_secs = 45"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl guard get GUARD-ECHO command\necho nested legacy"),
        "output: {}",
        output
    );
    assert!(
        output.contains("$ govctl guard get GUARD-ECHO timeout_secs\n45"),
        "output: {}",
        output
    );
    Ok(())
}
