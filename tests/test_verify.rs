//! Verification guard integration tests.

mod common;

use common::{TestResult, init_project, run_commands};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn test_verify_runs_project_default_guard() -> TestResult {
    let temp_dir = init_project()?;

    append_verification_config(temp_dir.path(), true, &["GUARD-ECHO"])?;
    write_guard(temp_dir.path(), "GUARD-ECHO", "true", None)?;

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
        1,
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
        1,
    )?;

    let output = run_command_with_timeout(temp_dir.path(), &["verify"], Duration::from_secs(2))?
        .ok_or("govctl verify hung while collecting output from a detached guard descendant")?;
    assert!(output.contains("PASS GUARD-DETACHED"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);

    Ok(())
}

#[test]
fn test_work_move_done_rejects_failed_required_guard() -> TestResult {
    let temp_dir = init_project()?;

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1", None)?;
    write_active_work_item(temp_dir.path(), "WI-2026-01-01-001", "GUARD-FAIL", None)?;

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

    write_guard(temp_dir.path(), "GUARD-FAIL", "exit 1", None)?;
    write_active_work_item(
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
            return Ok(Some(format_command_output(args, &result)));
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let _ = child.kill();
    let _ = child.wait();
    Ok(None)
}

#[cfg(unix)]
fn format_command_output(args: &[&str], result: &std::process::Output) -> String {
    let mut output = format!("$ govctl {}\n", args.join(" "));
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    if !stdout.is_empty() {
        output.push_str(&stdout);
        if !stdout.ends_with('\n') {
            output.push('\n');
        }
    }
    if !stderr.is_empty() {
        output.push_str(&stderr);
        if !stderr.ends_with('\n') {
            output.push('\n');
        }
    }

    output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    output
}

fn append_verification_config(
    dir: &Path,
    enabled: bool,
    guard_ids: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let existing = fs::read_to_string(&config_path)?;
    let default_guards = guard_ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let appended = format!(
        "{existing}\n[verification]\nenabled = {enabled}\ndefault_guards = [{default_guards}]\n"
    );
    fs::write(config_path, appended)?;
    Ok(())
}

fn write_guard(
    dir: &Path,
    guard_id: &str,
    command: &str,
    pattern: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    write_guard_with_timeout(dir, guard_id, command, pattern, 300)
}

fn write_guard_with_timeout(
    dir: &Path,
    guard_id: &str,
    command: &str,
    pattern: Option<&str>,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir
        .join("gov/guard")
        .join(format!("{}.toml", guard_id.to_lowercase()));
    let pattern_line = pattern
        .map(|pattern| format!("pattern = \"{pattern}\"\n"))
        .unwrap_or_default();
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\ntimeout_secs = {timeout_secs}\n{pattern_line}"
    );
    fs::write(path, content)?;
    Ok(())
}

fn write_active_work_item(
    dir: &Path,
    work_id: &str,
    guard_id: &str,
    waiver_reason: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join("gov/work/2026-01-01-guarded-item.toml");
    let waiver = waiver_reason
        .map(|reason| {
            format!("\n[[verification.waivers]]\nguard = \"{guard_id}\"\nreason = \"{reason}\"\n")
        })
        .unwrap_or_default();
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{work_id}\"\ntitle = \"Guarded Item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Guarded work item\"\n\n[[content.acceptance_criteria]]\ntext = \"done criteria\"\nstatus = \"done\"\ncategory = \"chore\"\n\n[verification]\nrequired_guards = [\"{guard_id}\"]{waiver}"
    );
    fs::write(path, content)?;
    Ok(())
}
