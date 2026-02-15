//! Process-level exclusive lock for the gov tree.
//!
//! Implements [[RFC-0004]] concurrent write safety: at most one write command
//! holds exclusive access at any time. Lock is released when the guard is dropped
//! (e.g. on process exit or when the command finishes).

use crate::config::Config;
use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::OpenOptions;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

/// Name of the lock file under the gov root (per ADR-0025).
const LOCK_FILE_NAME: &str = ".govctl.lock";

/// Backoff between try_lock attempts.
const POLL_INTERVAL_MS: u64 = 100;

/// Guard that holds the exclusive lock; releasing on drop.
pub struct GovLockGuard {
    _file: std::fs::File,
}

/// Acquires an exclusive lock on the gov root, waiting up to `timeout_secs`.
/// Returns a guard that releases the lock when dropped.
///
/// Fails with an actionable error if the lock cannot be acquired within the timeout.
pub fn acquire_gov_lock(config: &Config) -> Result<GovLockGuard> {
    let gov_root = config.paths.gov_root.as_path();
    let lock_path = gov_root.join(LOCK_FILE_NAME);
    let timeout_secs = config.concurrency.lock_timeout_secs;

    // Ensure gov root exists so we can create the lock file
    if !gov_root.exists() {
        anyhow::bail!(
            "Gov root does not exist: {}. Run 'govctl init' first.",
            gov_root.display()
        );
    }

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("Failed to open lock file: {}", lock_path.display()))?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let poll = Duration::from_millis(POLL_INTERVAL_MS);

    loop {
        match file.try_lock_exclusive() {
            Ok(()) => {
                return Ok(GovLockGuard { _file: file });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    anyhow::bail!(
                        "Another govctl write command is in progress. \
                         Wait for it to finish or retry later. \
                         (Timed out after {} seconds waiting for exclusive access.)",
                        timeout_secs
                    );
                }
                thread::sleep(poll);
            }
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Failed to acquire lock: {}", lock_path.display()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_file_name_is_under_gov_root() {
        assert_eq!(LOCK_FILE_NAME, ".govctl.lock");
    }
}
