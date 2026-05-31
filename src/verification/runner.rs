use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::GuardEntry;
use regex::RegexBuilder;
use std::io::{Read, Seek, SeekFrom};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

pub const DEFAULT_GUARD_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct GuardRunResult {
    pub id: String,
    pub passed: bool,
    pub timed_out: bool,
    pub primary_shell_running_at_timeout: Option<bool>,
    pub output: String,
}

pub fn run_guard(config: &Config, guard: &GuardEntry) -> Result<GuardRunResult, Diagnostic> {
    let project_root = config
        .gov_root
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| config.gov_root.clone());
    let timeout = if guard.spec.check.timeout_secs == 0 {
        DEFAULT_GUARD_TIMEOUT_SECS
    } else {
        guard.spec.check.timeout_secs
    };

    let mut stdout_capture = GuardOutputCapture::new(guard, "stdout")?;
    let mut stderr_capture = GuardOutputCapture::new(guard, "stderr")?;

    let mut command = Command::new("/bin/bash");
    command
        .args(["-lc", &guard.spec.check.command])
        .current_dir(project_root)
        .stdout(stdout_capture.stdio(guard, "stdout")?)
        .stderr(stderr_capture.stdio(guard, "stderr")?);
    configure_guard_process_group(&mut command);

    let mut child = command.spawn().map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E1004GuardCheckFailed,
            format!(
                "Failed to start verification guard '{}': {}",
                guard.meta().id,
                err
            ),
            guard.path.display().to_string(),
        )
    })?;
    let child_id = child.id();
    let process_group = guard_process_group(child_id);

    let deadline = Duration::from_secs(timeout);
    let started = Instant::now();
    let mut timed_out = false;
    let mut primary_shell_running_at_timeout = None;
    let status: ExitStatus;

    loop {
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                status = exit_status;
                terminate_guard_process_group(process_group);
                break;
            }
            Ok(None) if started.elapsed() < deadline => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Ok(None) => {
                timed_out = true;
                primary_shell_running_at_timeout = Some(true);
                terminate_guard_process(&mut child, process_group);
                status = child.wait().map_err(|err| {
                    Diagnostic::new(
                        DiagnosticCode::E1004GuardCheckFailed,
                        format!(
                            "Failed while waiting on timed-out verification guard '{}': {}",
                            guard.meta().id,
                            err
                        ),
                        guard.path.display().to_string(),
                    )
                })?;
                break;
            }
            Err(err) => {
                return Err(Diagnostic::new(
                    DiagnosticCode::E1004GuardCheckFailed,
                    format!(
                        "Failed while waiting on verification guard '{}': {}",
                        guard.meta().id,
                        err
                    ),
                    guard.path.display().to_string(),
                ));
            }
        }
    }

    let stdout = stdout_capture.read(guard, "stdout")?;
    let stderr = stderr_capture.read(guard, "stderr")?;
    let combined_output = format!(
        "{}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );

    let pattern_matched = match &guard.spec.check.pattern {
        Some(pattern) => RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .map_err(|err| {
                Diagnostic::new(
                    DiagnosticCode::E1001GuardSchemaInvalid,
                    format!(
                        "Invalid regex pattern for guard '{}': {}",
                        guard.meta().id,
                        err
                    ),
                    guard.path.display().to_string(),
                )
            })?
            .is_match(&combined_output),
        None => true,
    };

    Ok(GuardRunResult {
        id: guard.meta().id.clone(),
        passed: !timed_out && status.success() && pattern_matched,
        timed_out,
        primary_shell_running_at_timeout,
        output: combined_output,
    })
}

struct GuardOutputCapture {
    file: NamedTempFile,
}

impl GuardOutputCapture {
    fn new(guard: &GuardEntry, stream_name: &str) -> Result<Self, Diagnostic> {
        let file = NamedTempFile::new().map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to create {stream_name} capture file for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })?;
        Ok(Self { file })
    }

    fn stdio(&self, guard: &GuardEntry, stream_name: &str) -> Result<Stdio, Diagnostic> {
        self.file.reopen().map(Stdio::from).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to prepare {stream_name} capture file for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })
    }

    fn read(&mut self, guard: &GuardEntry, stream_name: &str) -> Result<Vec<u8>, Diagnostic> {
        let file = self.file.as_file_mut();
        file.seek(SeekFrom::Start(0)).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to rewind {stream_name} capture file for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })?;

        let mut output = Vec::new();
        file.read_to_end(&mut output).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to collect {stream_name} for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })?;
        Ok(output)
    }
}

#[cfg(unix)]
fn configure_guard_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_guard_process_group(_command: &mut Command) {}

#[cfg(unix)]
#[derive(Clone, Copy)]
struct GuardProcessGroup {
    pgid: i32,
}

#[cfg(not(unix))]
#[derive(Clone, Copy)]
struct GuardProcessGroup;

#[cfg(unix)]
fn guard_process_group(child_id: u32) -> Option<GuardProcessGroup> {
    let child_pid = i32::try_from(child_id).ok()?;
    let pgid = unix_getpgid(child_pid)?;
    let current_pgid = unix_getpgrp()?;

    if pgid == current_pgid || pgid != child_pid {
        return None;
    }

    Some(GuardProcessGroup { pgid })
}

#[cfg(not(unix))]
fn guard_process_group(_child_id: u32) -> Option<GuardProcessGroup> {
    None
}

fn terminate_guard_process(child: &mut Child, process_group: Option<GuardProcessGroup>) {
    terminate_guard_process_group(process_group);
    let _ = child.kill();
}

#[cfg(unix)]
fn terminate_guard_process_group(process_group: Option<GuardProcessGroup>) {
    let Some(process_group) = process_group else {
        return;
    };

    signal_process_group(process_group, SIGTERM);
    std::thread::sleep(Duration::from_millis(25));
    signal_process_group(process_group, SIGKILL);
}

#[cfg(unix)]
fn signal_process_group(process_group: GuardProcessGroup, signal: i32) {
    // Negative PID targets the process group. The group is captured after spawn
    // and rejected if it is not the isolated child group.
    unsafe {
        let _ = kill(-process_group.pgid, signal);
    }
}

#[cfg(not(unix))]
fn terminate_guard_process_group(_process_group: Option<GuardProcessGroup>) {}

#[cfg(unix)]
const SIGTERM: i32 = 15;

#[cfg(unix)]
const SIGKILL: i32 = 9;

#[cfg(unix)]
fn unix_getpgid(pid: i32) -> Option<i32> {
    let pgid = unsafe { getpgid(pid) };
    (pgid > 0).then_some(pgid)
}

#[cfg(unix)]
fn unix_getpgrp() -> Option<i32> {
    let pgid = unsafe { getpgrp() };
    (pgid > 0).then_some(pgid)
}

#[cfg(unix)]
unsafe extern "C" {
    fn getpgid(pid: i32) -> i32;
    fn getpgrp() -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(all(test, unix))]
mod unix_process_group_tests {
    use super::*;

    #[test]
    fn current_process_group_is_not_treated_as_guard_group() {
        assert!(guard_process_group(std::process::id()).is_none());
    }

    #[test]
    fn isolated_child_process_group_is_captured() -> Result<(), Box<dyn std::error::Error>> {
        let mut command = Command::new("/bin/sleep");
        command.arg("5");
        configure_guard_process_group(&mut command);

        let mut child = command.spawn()?;
        let process_group = guard_process_group(child.id());

        assert!(process_group.is_some());
        terminate_guard_process(&mut child, process_group);
        let _ = child.wait();

        Ok(())
    }
}
