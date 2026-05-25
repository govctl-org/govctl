//! Shared verification guard loading, validation, and execution.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{GuardEntry, WorkItemEntry};
use regex::RegexBuilder;
use std::collections::{HashMap, HashSet};
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

pub fn build_guard_index(
    guards: Vec<GuardEntry>,
) -> (HashMap<String, GuardEntry>, Vec<Diagnostic>) {
    let mut index = HashMap::new();
    let mut diagnostics = Vec::new();

    for guard in guards {
        let guard_id = guard.meta().id.clone();
        if let Some(existing) = index.insert(guard_id.clone(), guard.clone()) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1003GuardDuplicate,
                format!(
                    "Duplicate verification guard ID '{}': {} and {}",
                    guard_id,
                    existing.path.display(),
                    guard.path.display()
                ),
                guard.path.display().to_string(),
            ));
        }
    }

    (index, diagnostics)
}

pub fn configured_default_guard_ids(config: &Config) -> Vec<String> {
    if config.verification.enabled {
        dedup_guard_ids(config.verification.default_guards.iter().cloned())
    } else {
        vec![]
    }
}

pub fn required_guard_ids_before_waivers(
    config: &Config,
    work_item: &WorkItemEntry,
) -> Vec<String> {
    dedup_guard_ids(
        configured_default_guard_ids(config)
            .into_iter()
            .chain(work_item.spec.verification.required_guards.iter().cloned()),
    )
}

pub fn effective_required_guard_ids(config: &Config, work_item: &WorkItemEntry) -> Vec<String> {
    let waived: HashSet<&str> = work_item
        .spec
        .verification
        .waivers
        .iter()
        .map(|waiver| waiver.guard.as_str())
        .collect();

    required_guard_ids_before_waivers(config, work_item)
        .into_iter()
        .filter(|guard_id| !waived.contains(guard_id.as_str()))
        .collect()
}

pub fn validate_guard_configuration(
    config: &Config,
    guards: &HashMap<String, GuardEntry>,
    work_items: &[WorkItemEntry],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(validate_verification_config(config, guards));
    for work_item in work_items {
        diagnostics.extend(validate_work_item_verification(config, guards, work_item));
    }
    diagnostics
}

pub fn validate_verification_config(
    config: &Config,
    guards: &HashMap<String, GuardEntry>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for guard_id in &config.verification.default_guards {
        if !guards.contains_key(guard_id) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("verification.default_guards references unknown guard: {guard_id}"),
                "gov/config.toml",
            ));
        }
    }

    for guard in guards.values() {
        if let Some(pattern) = &guard.spec.check.pattern
            && RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .is_err()
        {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1001GuardSchemaInvalid,
                format!("Invalid guard regex pattern: {}", guard.meta().id),
                guard.path.display().to_string(),
            ));
        }
    }

    diagnostics
}

pub fn validate_work_item_verification(
    config: &Config,
    guards: &HashMap<String, GuardEntry>,
    work_item: &WorkItemEntry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for guard_id in &work_item.spec.verification.required_guards {
        if !guards.contains_key(guard_id) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1002GuardNotFound,
                format!("Work item references unknown verification guard: {guard_id}"),
                config.display_path(&work_item.path).display().to_string(),
            ));
        }
    }

    let effective_before_waivers: HashSet<String> =
        required_guard_ids_before_waivers(config, work_item)
            .into_iter()
            .collect();
    let mut seen_waivers = HashSet::new();
    for waiver in &work_item.spec.verification.waivers {
        let work_path = config.display_path(&work_item.path).display().to_string();
        if !seen_waivers.insert(waiver.guard.as_str()) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!("Duplicate waiver for verification guard: {}", waiver.guard),
                work_path.clone(),
            ));
        }
        if waiver.reason.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!(
                    "Verification waiver reason must not be empty: {}",
                    waiver.guard
                ),
                work_path.clone(),
            ));
        }
        if !guards.contains_key(&waiver.guard) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E1002GuardNotFound,
                format!(
                    "Work item waiver references unknown verification guard: {}",
                    waiver.guard
                ),
                work_path.clone(),
            ));
        }
        if !effective_before_waivers.contains(&waiver.guard) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!(
                    "Verification waiver references guard that is not required for this work item: {}",
                    waiver.guard
                ),
                work_path,
            ));
        }
    }

    diagnostics
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

pub fn unknown_guard_diagnostic(guard_id: &str, location: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1002GuardNotFound,
        format!("Unknown verification guard: {guard_id}"),
        location,
    )
}

fn dedup_guard_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for id in ids {
        if seen.insert(id.clone()) {
            deduped.push(id);
        }
    }

    deduped
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
