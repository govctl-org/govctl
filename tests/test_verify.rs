//! Verification guard integration tests.

mod common;

use common::{
    TestResult, append_verification_config, init_project, run_commands,
    write_canonical_guarded_work_item, write_guard_with_timeout,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const NON_TIMEOUT_GUARD_TIMEOUT_SECS: u64 = 10;

#[cfg(unix)]
const DETACHED_GUARD_RUN_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn test_verify_runs_project_default_guard() -> TestResult {
    let temp_dir = init_project()?;

    append_verification_config(temp_dir.path(), true, &["GUARD-ECHO"])?;
    write_guard_with_timeout(temp_dir.path(), "GUARD-ECHO", "true", None, 300)?;

    let output = run_commands(temp_dir.path(), &[&["verify"]])?;
    assert!(output.contains("PASS GUARD-ECHO"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);

    Ok(())
}

#[test]
fn test_verify_noisy_guard_output_does_not_timeout() -> TestResult {
    let temp_dir = init_project()?;

    append_verification_config(temp_dir.path(), true, &["GUARD-NOISY"])?;
    write_guard_with_timeout(
        temp_dir.path(),
        "GUARD-NOISY",
        "yes noisy | head -c 131072",
        None,
        NON_TIMEOUT_GUARD_TIMEOUT_SECS,
    )?;

    let output = run_commands(temp_dir.path(), &[&["verify"]])?;
    assert!(output.contains("PASS GUARD-NOISY"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);

    Ok(())
}

#[test]
fn test_verify_timeout_diagnostic_mentions_primary_shell_state() -> TestResult {
    let temp_dir = init_project()?;

    append_verification_config(temp_dir.path(), true, &["GUARD-SLEEP"])?;
    write_guard_with_timeout(temp_dir.path(), "GUARD-SLEEP", "sleep 2", None, 1)?;

    let output = run_commands(temp_dir.path(), &[&["verify"]])?;
    assert!(
        output.contains("primary shell process was still running when timeout handling began"),
        "output: {}",
        output
    );
    assert!(output.contains("exit: 1"), "output: {}", output);

    Ok(())
}

#[cfg(unix)]
#[test]
fn test_verify_detached_guard_descendant_holding_output_open_does_not_hang() -> TestResult {
    if !perl_supports_setsid() {
        return Ok(());
    }

    let temp_dir = init_project()?;

    append_verification_config(temp_dir.path(), true, &["GUARD-DETACHED"])?;
    write_guard_with_timeout(
        temp_dir.path(),
        "GUARD-DETACHED",
        "perl -MPOSIX=setsid -e 'if (fork() == 0) { setsid(); sleep 6 }'",
        None,
        NON_TIMEOUT_GUARD_TIMEOUT_SECS,
    )?;

    let output =
        run_command_with_timeout(temp_dir.path(), &["verify"], DETACHED_GUARD_RUN_TIMEOUT)?
            .ok_or("govctl verify hung while collecting output from a detached guard descendant")?;
    assert!(output.contains("PASS GUARD-DETACHED"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);

    Ok(())
}

#[test]
fn test_work_move_done_rejects_failed_required_guard() -> TestResult {
    let temp_dir = init_project()?;

    write_guard_with_timeout(temp_dir.path(), "GUARD-FAIL", "exit 1", None, 300)?;
    write_canonical_guarded_work_item(temp_dir.path(), "WI-2026-01-01-001", "GUARD-FAIL", None)?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-2026-01-01-001", "done"]],
    )?;
    assert!(
        output.contains("Cannot mark as done: verification guard requirements failed"),
        "output: {}",
        output
    );
    assert!(output.contains("error[E1004]"), "output: {}", output);
    assert!(output.contains("exit: 1"), "output: {}", output);

    Ok(())
}

#[test]
fn test_work_move_done_allows_waived_guard() -> TestResult {
    let temp_dir = init_project()?;

    write_guard_with_timeout(temp_dir.path(), "GUARD-FAIL", "exit 1", None, 300)?;
    write_canonical_guarded_work_item(
        temp_dir.path(),
        "WI-2026-01-01-001",
        "GUARD-FAIL",
        Some("Guard does not apply to this work item."),
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "move", "WI-2026-01-01-001", "done"]],
    )?;
    assert!(output.contains("exit: 0"), "output: {}", output);

    let work_file = temp_dir
        .path()
        .join("gov/work/2026-01-01-guarded-item.toml");
    let content = fs::read_to_string(&work_file)?;
    assert!(
        content.contains("status = \"done\""),
        "content: {}",
        content
    );

    Ok(())
}

#[cfg(unix)]
fn perl_supports_setsid() -> bool {
    Command::new("perl")
        .args(["-MPOSIX=setsid", "-e", "setsid(); exit 0"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(unix)]
fn run_command_with_timeout(
    dir: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let started = Instant::now();
    while started.elapsed() < timeout {
        if child.try_wait()?.is_some() {
            let result = child.wait_with_output()?;
            return Ok(Some(common::format_command_output(args, &result)));
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let _ = child.kill();
    let _ = child.wait();
    Ok(None)
}
