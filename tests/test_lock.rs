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

mod lock_tests;
