//! Lock integration tests - verify file lock behavior for concurrent writes.
//!
//! Tests RFC-0004 concurrent write safety: at most one write command holds exclusive access.

mod common;

use common::{init_project, run_commands, today};
use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

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
    let lock_path = temp_dir.path().join("gov/.govctl.lock");
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

/// Test: Lock file is created under gov root
#[test]
fn test_lock_file_location() -> common::TestResult {
    let temp_dir = init_project()?;

    let lock_path = temp_dir.path().join("gov/.govctl.lock");

    // init_project runs `govctl init` which is a write command,
    // so lock file may already exist. Remove it first.
    let _ = fs::remove_file(&lock_path);

    // After a write command, lock file exists (even if released)
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]])?;

    assert!(
        lock_path.exists(),
        "Lock file should be created under gov root"
    );
    Ok(())
}

/// Test: Read-only commands don't create lock
#[test]
fn test_read_commands_no_lock() -> common::TestResult {
    let temp_dir = init_project()?;
    let _date = today();

    let lock_path = temp_dir.path().join("gov/.govctl.lock");

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
    Ok(())
}

/// Test: Lock timeout configuration is respected
#[test]
fn test_lock_timeout_configurable() -> common::TestResult {
    let temp_dir = init_project()?;

    // Create config with short timeout
    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
docs_output = "docs"

[concurrency]
lock_timeout_secs = 1
"#;
    fs::write(&config_path, config_content)?;

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

    let lock_path = temp_dir.path().join("gov/.govctl.lock");

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

/// Helper: Kill a process, waiting with try_wait polling
/// (workaround for macOS wait_timeout issues)
fn kill_and_wait(mut child: std::process::Child, timeout: Duration) {
    let _ = child.kill();

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return,
        }
    }
    let _ = child.kill();
}

/// Create a config file with specified lock timeout
fn create_config_with_timeout(
    temp_dir: &std::path::Path,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = temp_dir.join("gov/config.toml");
    let config_content = format!(
        r#"[project]
name = "test-project"

[paths]
docs_output = "docs"

[concurrency]
lock_timeout_secs = {}
"#,
        timeout_secs
    );
    fs::write(&config_path, config_content)?;
    Ok(())
}

/// Test: Concurrent write is blocked by lock (cross-process)
///
/// Uses govctl itself as the lock holder - spawns a write command that
/// blocks waiting for user input (which never comes), holding the lock.
#[test]
fn test_concurrent_write_blocked_by_lock() -> common::TestResult {
    let temp_dir = init_project()?;

    // Short timeout for the second writer
    create_config_with_timeout(temp_dir.path(), 1)?;

    // Start a work item deletion in another process
    // This will prompt for confirmation, holding the lock while waiting
    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir)?;
    let work_file = work_dir.join("2026-01-01-test-item.toml");
    fs::write(
        &work_file,
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Test Item"
status = "queue"
created = "2026-01-01"

[content]
description = "Test"
acceptance_criteria = []
"#,
    )?;

    // Spawn a delete command (will hold lock while waiting for confirmation)
    let holder = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "delete", "WI-2026-01-01-001"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    // Wait a bit for the holder to acquire the lock
    thread::sleep(Duration::from_millis(500));

    // Now try another write - should timeout
    let start = Instant::now();
    let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "new", "Should timeout"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .output()?;
    let elapsed = start.elapsed();

    // Should have timed out quickly
    assert!(
        elapsed < Duration::from_secs(10),
        "Command should have timed out quickly, but took {:?}",
        elapsed
    );

    // The command should have failed
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Another govctl write command is in progress")
            || stderr.contains("Timed out"),
        "Expected timeout error, got: {}",
        stderr
    );

    // Clean up
    kill_and_wait(holder, Duration::from_secs(2));
    Ok(())
}

/// Test: Concurrent write succeeds after lock is released
#[test]
fn test_write_succeeds_after_lock_released() -> common::TestResult {
    let temp_dir = init_project()?;

    // Longer timeout for this test
    create_config_with_timeout(temp_dir.path(), 30)?;

    // Create a work item to delete
    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir)?;
    let work_file = work_dir.join("2026-01-01-test-item.toml");
    fs::write(
        &work_file,
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Test Item"
status = "queue"
created = "2026-01-01"

[content]
description = "Test"
acceptance_criteria = []
"#,
    )?;

    // Spawn a delete command with -f flag (no confirmation, completes immediately)
    let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "delete", "WI-2026-01-01-001", "-f"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .status()?;

    assert!(result.success(), "Delete should succeed");

    // Now a write should succeed immediately
    let start = Instant::now();
    let output = run_commands(temp_dir.path(), &[&["work", "new", "After release"]])?;
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "Write should succeed immediately, took {:?}",
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

#[test]
fn test_concurrent_tick_commands_persist_all_acceptance_criteria_updates() -> common::TestResult {
    let temp_dir = init_project()?;
    create_config_with_timeout(temp_dir.path(), 30)?;

    let today = today();
    let wi_id = format!("WI-{today}-001");

    let create_output = run_commands(
        temp_dir.path(),
        &[&["work", "new", "Concurrent tick persistence", "--active"]],
    )?;
    assert!(
        create_output.contains(&wi_id),
        "expected work item id in output: {create_output}"
    );

    let setup_output = run_commands(
        temp_dir.path(),
        &[
            &[
                "work",
                "add",
                wi_id.as_str(),
                "acceptance_criteria",
                "test: criterion one",
            ],
            &[
                "work",
                "add",
                wi_id.as_str(),
                "acceptance_criteria",
                "test: criterion two",
            ],
            &[
                "work",
                "add",
                wi_id.as_str(),
                "acceptance_criteria",
                "test: criterion three",
            ],
        ],
    )?;
    assert!(
        setup_output.contains("exit: 0"),
        "setup output: {setup_output}"
    );

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for index in 0..3 {
        let dir = temp_dir.path().to_path_buf();
        let wi_id = wi_id.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            Command::new(env!("CARGO_BIN_EXE_govctl"))
                .args([
                    "work",
                    "edit",
                    &wi_id,
                    &format!("acceptance_criteria[{index}]"),
                    "--tick",
                    "done",
                ])
                .current_dir(dir)
                .env("NO_COLOR", "1")
                .env("GOVCTL_DEFAULT_OWNER", "@test-user")
                .output()
        }));
    }

    for handle in handles {
        let output = handle
            .join()
            .map_err(|_| "tick thread panicked")?
            .map_err(|e| format!("failed to run concurrent tick command: {e}"))?;
        assert!(
            output.status.success(),
            "concurrent tick failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let get_output = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "show", &wi_id, "-o", "json"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()?;
    assert!(
        get_output.status.success(),
        "work show failed: stdout={} stderr={}",
        String::from_utf8_lossy(&get_output.stdout),
        String::from_utf8_lossy(&get_output.stderr)
    );
    let work: Value = serde_json::from_slice(&get_output.stdout)?;
    let criteria = work["content"]["acceptance_criteria"]
        .as_array()
        .ok_or("acceptance_criteria array missing or not an array")?;
    let done_count = criteria
        .iter()
        .filter(|item| item["status"] == "done")
        .count();
    assert_eq!(
        done_count,
        3,
        "expected all criteria to persist as done, got:\n{}",
        String::from_utf8_lossy(&get_output.stdout)
    );
    Ok(())
}
