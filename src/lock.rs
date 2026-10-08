//! Process-level exclusive lock for the gov tree.
//!
//! Implements [[RFC-0004]] concurrent write safety: at most one write command
//! holds exclusive access at any time. Lock is released when the guard is dropped
//! (e.g. on process exit or when the command finishes).

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use fs2::FileExt;
use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

/// Name of the lock file inside the workspace-local `.govctl/` directory —
/// [[ADR-0065]]. The legacy `gov/.govctl.lock` is neither used nor deleted.
pub(crate) const LOCK_FILE_NAME: &str = "gov.lock";

/// Backoff between try_lock attempts.
const POLL_INTERVAL_MS: u64 = 100;

/// Path of the gov-root write lock: one lock per governed project, kept in
/// workspace-local state rather than under `gov/` — [[ADR-0065]].
pub(crate) fn gov_lock_path(config: &Config) -> PathBuf {
    crate::local_index::project_root(config)
        .join(".govctl")
        .join(LOCK_FILE_NAME)
}

/// Guard that holds the exclusive lock; releasing on drop.
pub struct GovLockGuard {
    _file: std::fs::File,
}

/// Acquires an exclusive lock on the gov root, waiting up to `timeout_secs`.
/// Returns a guard that releases the lock when dropped.
///
/// Fails with an actionable error if the lock cannot be acquired within the timeout.
pub fn acquire_gov_lock(config: &Config) -> DiagnosticResult<GovLockGuard> {
    let gov_root = config.gov_root.as_path();
    let lock_path = gov_lock_path(config);
    let timeout_secs = config.concurrency.lock_timeout_secs;

    // Ensure gov root exists so we can create the lock file
    if !gov_root.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0502PathNotFound,
            format!(
                "Gov root does not exist: {}. Run 'govctl init' first.",
                gov_root.display()
            ),
            gov_root.display().to_string(),
        ));
    }

    if let Some(state_dir) = lock_path.parent() {
        std::fs::create_dir_all(state_dir).map_err(|e| {
            Diagnostic::io_error(
                "create local state directory",
                e,
                state_dir.display().to_string(),
            )
        })?;
    }

    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| Diagnostic::io_error("open lock file", e, lock_path.display().to_string()))?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let poll = Duration::from_millis(POLL_INTERVAL_MS);

    loop {
        match file.try_lock_exclusive() {
            Ok(()) => {
                return Ok(GovLockGuard { _file: file });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0503LockTimeout,
                        format!(
                            "Another govctl write command is in progress. Wait for it to finish or retry later. (Timed out after {} seconds waiting for exclusive access.)",
                            timeout_secs
                        ),
                        lock_path.display().to_string(),
                    ));
                }
                thread::sleep(poll);
            }
            Err(e) => {
                return Err(Diagnostic::io_error(
                    "acquire lock",
                    e,
                    lock_path.display().to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_lives_in_workspace_local_state() {
        let mut config = Config::default();
        config.gov_root = PathBuf::from("/project/gov");
        assert_eq!(
            gov_lock_path(&config),
            PathBuf::from("/project/.govctl/gov.lock")
        );
    }
}
