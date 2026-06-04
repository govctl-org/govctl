use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::GuardEntry;
use regex::RegexBuilder;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

mod capture;
mod process_group;

use capture::GuardOutputCapture;
use process_group::{
    configure_guard_process_group, guard_process_group, terminate_guard_process,
    terminate_guard_process_group,
};

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
