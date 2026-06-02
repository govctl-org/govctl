use super::*;

#[test]
fn test_guard_list() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ALPHA", "true")?;
    let long_command = "echo 1234567890 1234567890 1234567890 1234567890 1234567890";
    write_guard(temp_dir.path(), "GUARD-BETA", long_command)?;

    let output = run_commands(temp_dir.path(), &[&["guard", "list"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-ALPHA"), "output: {}", output);
    assert!(output.contains("GUARD-BETA"), "output: {}", output);
    let truncated_command = format!("{}…", long_command.chars().take(50).collect::<String>());
    assert!(
        output.contains(&truncated_command),
        "guard list should truncate long command: {}",
        output
    );
    assert!(
        !output.contains(long_command),
        "guard list should not print full long command: {}",
        output
    );
    Ok(())
}

#[test]
fn test_guard_show() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello")?;

    let output = run_commands(temp_dir.path(), &[&["guard", "show", "GUARD-ECHO"]])?;
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(output.contains("GUARD-ECHO"), "output: {}", output);
    assert!(output.contains("echo hello"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_show_json_output() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello")?;

    let output = run_commands(
        temp_dir.path(),
        &[&["guard", "show", "GUARD-ECHO", "-o", "json"]],
    )?;
    assert!(
        output.contains("\"id\": \"GUARD-ECHO\""),
        "output: {}",
        output
    );
    assert!(
        output.contains("\"command\": \"echo hello\""),
        "output: {}",
        output
    );
    assert!(output.contains("exit: 0"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_show_missing_returns_coded_error() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["guard", "show", "GUARD-MISSING"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E1002]"), "output: {}", output);
    Ok(())
}

#[test]
fn test_guard_get_field() -> common::TestResult {
    let temp_dir = init_project()?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "echo hello")?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &["guard", "get", "GUARD-ECHO", "command"],
            &["guard", "get", "GUARD-ECHO", "title"],
        ],
    )?;
    assert!(output.contains("echo hello"), "output: {}", output);
    assert!(output.contains("GUARD-ECHO"), "output: {}", output);
    Ok(())
}
