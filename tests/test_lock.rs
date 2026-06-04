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

fn write_queue_work_item_for_lock_delete(
    temp_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let work_dir = temp_dir.join("gov/work");
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
    Ok(())
}

mod lock_tests;
