use super::*;

/// Test: Write command creates and releases lock
#[test]
fn test_write_command_creates_lock_file() -> common::TestResult {
    let temp_dir = init_project()?;
    let _date = today();

    // Run a write command
    let output = run_commands(
        temp_dir.path(),
        &[&["work", "new", "Test work item", "--active"]],
    )?;

    // Verify command succeeded
    assert!(output.contains("Created work item"));

    // Lock file is released after command finishes, but the file exists
    let lock_path = temp_dir.path().join(".govctl/gov.lock");
    assert!(
        lock_path.exists(),
        "Lock file should exist after write command"
    );
    Ok(())
}

/// Test: Multiple sequential write commands work (lock is released between commands)
#[test]
fn test_sequential_write_commands_succeed() -> common::TestResult {
    let temp_dir = init_project()?;
    let _date = today();

    // Run multiple write commands sequentially
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "First work item"],
            &["work", "new", "Second work item"],
        ],
    )?;

    // Both should succeed
    assert!(output.contains("Created work item"));
    assert!(output.contains("exit: 0"));
    Ok(())
}

/// Test: Lock file is created in workspace-local state, not under gov root
/// ([[ADR-0065]])
#[test]
fn test_lock_file_location() -> common::TestResult {
    let temp_dir = init_project()?;

    let lock_path = temp_dir.path().join(".govctl/gov.lock");

    // init_project runs `govctl init` which is a write command,
    // so lock file may already exist. Remove it first.
    let _ = fs::remove_file(&lock_path);

    // After a write command, lock file exists (even if released)
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]])?;

    assert!(
        lock_path.exists(),
        "Lock file should be created under .govctl/"
    );
    assert!(
        !temp_dir.path().join("gov/.govctl.lock").exists(),
        "Write commands must not create a lock file under gov/"
    );
    Ok(())
}

/// Test: a missing `.govctl/` directory is created by the first write, and a
/// legacy `gov/.govctl.lock` is left untouched ([[ADR-0065]])
#[test]
fn test_lock_creates_state_dir_and_leaves_legacy_lock() -> common::TestResult {
    let temp_dir = init_project()?;
    let state_dir = temp_dir.path().join(".govctl");
    let _ = fs::remove_dir_all(&state_dir);
    let legacy = temp_dir.path().join("gov/.govctl.lock");
    fs::write(&legacy, "")?;

    let output = run_commands(temp_dir.path(), &[&["work", "new", "Test"]])?;
    assert!(output.contains("exit: 0"), "output: {output}");

    assert!(
        state_dir.join("gov.lock").exists(),
        "lock created under .govctl/"
    );
    assert!(
        legacy.exists(),
        "legacy gov/.govctl.lock must not be deleted"
    );
    Ok(())
}

/// Test: Read-only commands don't create lock
#[test]
fn test_read_commands_no_lock() -> common::TestResult {
    let temp_dir = init_project()?;
    let _date = today();

    let lock_path = temp_dir.path().join(".govctl/gov.lock");

    // Remove any existing lock file first
    let _ = fs::remove_file(&lock_path);

    // Run read-only commands
    run_commands(temp_dir.path(), &[&["status"]])?;

    // Read commands don't create the lock file
    assert!(
        !lock_path.exists(),
        "Read commands should not create lock file"
    );

    run_commands(temp_dir.path(), &[&["check"]])?;

    // Still no lock file
    assert!(
        !lock_path.exists(),
        "Read commands should not create lock file"
    );

    run_commands(temp_dir.path(), &[&["loop", "list"]])?;

    // Loop list is local state inspection and must remain lock-free.
    assert!(!lock_path.exists(), "Loop list should not create lock file");
    Ok(())
}

/// Test: Lock timeout configuration is respected
#[test]
fn test_lock_timeout_configurable() -> common::TestResult {
    let temp_dir = init_project()?;

    // Create config with short timeout
    create_config_with_timeout(temp_dir.path(), 1)?;

    // The timeout is now 1 second instead of default 30
    // A write command should still succeed quickly
    let output = run_commands(temp_dir.path(), &[&["work", "new", "Test"]])?;
    assert!(output.contains("Created work item"));
    Ok(())
}

/// Test: Lock is released after write command completes
#[test]
fn test_lock_released_after_write() -> common::TestResult {
    let temp_dir = init_project()?;

    let lock_path = temp_dir.path().join(".govctl/gov.lock");

    // Remove any existing lock
    let _ = fs::remove_file(&lock_path);

    // Run a write command
    run_commands(temp_dir.path(), &[&["work", "new", "Test"]])?;

    // Lock file should exist but be unlocked
    assert!(lock_path.exists(), "Lock file should exist");

    // Another write command should succeed immediately (lock was released)
    let start = Instant::now();
    let output = run_commands(temp_dir.path(), &[&["work", "new", "Test2"]])?;
    let elapsed = start.elapsed();

    // Should succeed quickly (not waiting for lock)
    assert!(
        elapsed < Duration::from_secs(2),
        "Second write should succeed immediately, took {:?}",
        elapsed
    );
    assert!(output.contains("Created work item"));
    Ok(())
}

#[test]
fn test_write_command_without_init_reports_missing_gov_root() -> common::TestResult {
    let temp_dir = tempfile::TempDir::new()?;

    let output = run_commands(temp_dir.path(), &[&["work", "new", "Needs init"]])?;
    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(output.contains("error[E0502]"), "output: {}", output);
    assert!(
        output.contains("Run 'govctl init' first"),
        "output: {}",
        output
    );
    Ok(())
}
