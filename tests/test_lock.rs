//! Lock integration tests - verify file lock behavior for concurrent writes.
//!
//! Tests RFC-0004 concurrent write safety: at most one write command holds exclusive access.

mod common;

use common::{init_project, run_commands, today};
use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Test: Write command creates and releases lock
#[test]
fn test_write_command_creates_lock_file() {
    let temp_dir = init_project();
    let _date = today();

    // Run a write command
    let output = run_commands(
        temp_dir.path(),
        &[&["work", "new", "Test work item", "--active"]],
    );

    // Verify command succeeded
    assert!(output.contains("Created work item"));

    // Lock file is released after command finishes, but the file exists
    let lock_path = temp_dir.path().join("gov/.govctl.lock");
    assert!(
        lock_path.exists(),
        "Lock file should exist after write command"
    );
}

/// Test: Multiple sequential write commands work (lock is released between commands)
#[test]
fn test_sequential_write_commands_succeed() {
    let temp_dir = init_project();
    let _date = today();

    // Run multiple write commands sequentially
    let output = run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "First work item"],
            &["work", "new", "Second work item"],
        ],
    );

    // Both should succeed
    assert!(output.contains("Created work item"));
    assert!(output.contains("exit: 0"));
}

/// Test: Lock file is created under gov root
#[test]
fn test_lock_file_location() {
    let temp_dir = init_project();

    let lock_path = temp_dir.path().join("gov/.govctl.lock");

    // init_project runs `govctl init` which is a write command,
    // so lock file may already exist. Remove it first.
    let _ = fs::remove_file(&lock_path);

    // After a write command, lock file exists (even if released)
    run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]]);

    assert!(
        lock_path.exists(),
        "Lock file should be created under gov root"
    );
}

/// Test: Read-only commands don't create lock
#[test]
fn test_read_commands_no_lock() {
    let temp_dir = init_project();
    let _date = today();

    let lock_path = temp_dir.path().join("gov/.govctl.lock");

    // Remove any existing lock file first
    let _ = fs::remove_file(&lock_path);

    // Run read-only commands
    run_commands(temp_dir.path(), &[&["status"]]);

    // Read commands don't create the lock file
    assert!(
        !lock_path.exists(),
        "Read commands should not create lock file"
    );

    run_commands(temp_dir.path(), &[&["check"]]);

    // Still no lock file
    assert!(
        !lock_path.exists(),
        "Read commands should not create lock file"
    );
}

/// Test: Lock timeout configuration is respected
#[test]
fn test_lock_timeout_configurable() {
    let temp_dir = init_project();

    // Create config with short timeout
    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
gov_root = "gov"
docs_output = "docs"

[concurrency]
lock_timeout_secs = 1
"#;
    fs::write(&config_path, config_content).unwrap();

    // The timeout is now 1 second instead of default 30
    // A write command should still succeed quickly
    let output = run_commands(temp_dir.path(), &[&["work", "new", "Test"]]);
    assert!(output.contains("Created work item"));
}

/// Test: Lock is released after write command completes
#[test]
fn test_lock_released_after_write() {
    let temp_dir = init_project();

    let lock_path = temp_dir.path().join("gov/.govctl.lock");

    // Remove any existing lock
    let _ = fs::remove_file(&lock_path);

    // Run a write command
    run_commands(temp_dir.path(), &[&["work", "new", "Test"]]);

    // Lock file should exist but be unlocked
    assert!(lock_path.exists(), "Lock file should exist");

    // Another write command should succeed immediately (lock was released)
    let start = Instant::now();
    let output = run_commands(temp_dir.path(), &[&["work", "new", "Test2"]]);
    let elapsed = start.elapsed();

    // Should succeed quickly (not waiting for lock)
    assert!(
        elapsed < Duration::from_secs(2),
        "Second write should succeed immediately, took {:?}",
        elapsed
    );
    assert!(output.contains("Created work item"));
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
fn create_config_with_timeout(temp_dir: &std::path::Path, timeout_secs: u64) {
    let config_path = temp_dir.join("gov/config.toml");
    let config_content = format!(
        r#"[project]
name = "test-project"

[paths]
gov_root = "gov"
docs_output = "docs"

[concurrency]
lock_timeout_secs = {}
"#,
        timeout_secs
    );
    fs::write(&config_path, config_content).unwrap();
}

/// Test: Concurrent write is blocked by lock (cross-process)
///
/// Uses govctl itself as the lock holder - spawns a write command that
/// blocks waiting for user input (which never comes), holding the lock.
#[test]
fn test_concurrent_write_blocked_by_lock() {
    let temp_dir = init_project();

    // Short timeout for the second writer
    create_config_with_timeout(temp_dir.path(), 1);

    // Start a work item deletion in another process
    // This will prompt for confirmation, holding the lock while waiting
    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir).unwrap();
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
    )
    .unwrap();

    // Spawn a delete command (will hold lock while waiting for confirmation)
    let holder = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "delete", "WI-2026-01-01-001"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start lock holder");

    // Wait a bit for the holder to acquire the lock
    thread::sleep(Duration::from_millis(500));

    // Now try another write - should timeout
    let start = Instant::now();
    let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "new", "Should timeout"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to run govctl");
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
}

/// Test: Concurrent write succeeds after lock is released
#[test]
fn test_write_succeeds_after_lock_released() {
    let temp_dir = init_project();

    // Longer timeout for this test
    create_config_with_timeout(temp_dir.path(), 30);

    // Create a work item to delete
    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir).unwrap();
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
    )
    .unwrap();

    // Spawn a delete command with -f flag (no confirmation, completes immediately)
    let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "delete", "WI-2026-01-01-001", "-f"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .status()
        .expect("Failed to run govctl");

    assert!(result.success(), "Delete should succeed");

    // Now a write should succeed immediately
    let start = Instant::now();
    let output = run_commands(temp_dir.path(), &[&["work", "new", "After release"]]);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "Write should succeed immediately, took {:?}",
        elapsed
    );
    assert!(output.contains("Created work item"));
}
