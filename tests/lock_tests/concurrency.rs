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
