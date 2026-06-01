//! Common helpers shared across integration test binaries.

#![allow(dead_code)] // Functions used across different test binaries

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub mod loop_helpers;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

pub fn current_test_snapshot_name(prefix: &str, function_name: &str) -> String {
    let test_name = function_name.rsplit("::").next().unwrap_or(function_name);
    let snapshot_case = test_name.strip_prefix("test_").unwrap_or(test_name);
    format!("{prefix}__{snapshot_case}")
}

pub fn named_snapshot_name(prefix: &str, name: &str) -> String {
    format!("{prefix}__{name}")
}

#[macro_export]
macro_rules! with_test_snapshot_settings {
    ($body:block) => {{
        insta::with_settings!({
            snapshot_path => $crate::common::snapshot_path(),
            prepend_module_to_snapshot => false
        }, $body);
    }};
}

/// Get today's date in YYYY-MM-DD format (same as govctl uses)
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Normalize output for stable snapshots:
/// - Replace temp directory paths with `<TEMPDIR>`
/// - Replace today's date with `<DATE>`
/// - Replace work item IDs (WI-YYYY-MM-DD-NNN) with WI-<DATE>-NNN
/// - Replace ADR IDs with date component normalized
pub fn normalize_output(output: &str, dir: &Path, date: &str) -> Result<String, regex::Error> {
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let canonical_str = canonical.display().to_string();
    let dir_str = dir.display().to_string();
    let mut normalized = output.replace(&canonical_str, "<TEMPDIR>");
    normalized = normalized.replace(&dir_str, "<TEMPDIR>");
    normalized = normalized.replace(date, "<DATE>");

    // Replace work item IDs
    let wi_pattern = regex::Regex::new(r"WI-\d{4}-\d{2}-\d{2}-(\d{3})")?;
    normalized = wi_pattern
        .replace_all(&normalized, "WI-<DATE>-$1")
        .to_string();

    // Replace ADR filenames with dates
    let adr_file_pattern = regex::Regex::new(r"ADR-(\d{4})-")?;
    normalized = adr_file_pattern
        .replace_all(&normalized, "ADR-XXXX-")
        .to_string();

    // Replace signature hashes (date-dependent due to embedded dates in specs)
    let sig_pattern = regex::Regex::new(r"sha256:[0-9a-f]{64}")?;
    normalized = sig_pattern
        .replace_all(&normalized, "sha256:<HASH>")
        .to_string();

    // Replace govctl version in JSON contexts to avoid snapshot churn on version bumps.
    // Only replace inside double quotes to avoid corrupting semver strings in CHANGELOG fixtures.
    let version = env!("CARGO_PKG_VERSION");
    normalized = normalized.replace(&format!("\"{version}\""), "\"<VERSION>\"");

    Ok(normalized)
}

/// Run govctl commands in a directory and capture output.
pub fn run_commands(dir: &Path, commands: &[&[&str]]) -> Result<String, std::io::Error> {
    let mut output = String::new();

    for args in commands {
        append_command_output(dir, args, &mut output)?;
    }

    Ok(output)
}

/// Run commands with dynamic String arguments (for work item IDs with dates)
pub fn run_dynamic_commands(
    dir: &Path,
    commands: &[Vec<String>],
) -> Result<String, std::io::Error> {
    let mut output = String::new();

    for args in commands {
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        append_command_output(dir, &args_str, &mut output)?;
    }

    Ok(output)
}

fn append_command_output(
    dir: &Path,
    args: &[&str],
    output: &mut String,
) -> Result<(), std::io::Error> {
    let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()?;

    output.push_str(&format_command_output(args, &result));

    Ok(())
}

pub fn format_command_output(args: &[&str], result: &std::process::Output) -> String {
    let mut output = format!("$ govctl {}\n", args.join(" "));
    append_process_output(&mut output, &result.stdout);
    append_process_output(&mut output, &result.stderr);
    output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    output
}

fn append_process_output(output: &mut String, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    if !text.is_empty() {
        output.push_str(&text);
        if !text.ends_with('\n') {
            output.push('\n');
        }
    }
}

/// Initialize a govctl project in a temp directory.
///
/// If `schema_version` is provided, overrides the config schema version
/// (used by migration tests to simulate older repositories).
pub fn init_project_at(schema_version: Option<u32>) -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_govctl"));
    cmd.args(["init"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user");

    if let Some(v) = schema_version {
        cmd.env("GOVCTL_SCHEMA_VERSION", v.to_string());
    }

    let result = cmd.output()?;
    assert!(result.status.success(), "govctl init failed");
    Ok(temp_dir)
}

pub fn init_project() -> Result<TempDir, Box<dyn std::error::Error>> {
    init_project_at(None)
}

pub fn append_verification_config(
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

pub fn write_guard(
    dir: &Path,
    guard_id: &str,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = format!(
        "#:schema ../schema/guard.schema.json\n\n[govctl]\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\n"
    );
    write_guard_content(dir, guard_id, &content)
}

pub fn write_guard_with_timeout(
    dir: &Path,
    guard_id: &str,
    command: &str,
    pattern: Option<&str>,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let pattern_line = pattern
        .map(|pattern| format!("pattern = \"{pattern}\"\n"))
        .unwrap_or_default();
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\ntimeout_secs = {timeout_secs}\n{pattern_line}"
    );
    write_guard_content(dir, guard_id, &content)
}

fn write_guard_content(
    dir: &Path,
    guard_id: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir
        .join("gov/guard")
        .join(format!("{}.toml", guard_id.to_lowercase()));
    fs::write(path, content)?;
    Ok(())
}

pub fn write_guarded_work_item(
    dir: &Path,
    work_id: &str,
    guard_id: &str,
    waiver_reason: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let waiver = guard_waiver(guard_id, waiver_reason);
    let content = format!(
        "#:schema ../schema/work.schema.json\n\n[govctl]\nid = \"{work_id}\"\ntitle = \"Guarded Item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Guarded work item\"\n\n[[content.acceptance_criteria]]\ntext = \"done\"\nstatus = \"done\"\ncategory = \"chore\"\n\n[verification]\nrequired_guards = [\"{guard_id}\"]{waiver}"
    );
    write_guarded_work_item_content(dir, &content)
}

pub fn write_canonical_guarded_work_item(
    dir: &Path,
    work_id: &str,
    guard_id: &str,
    waiver_reason: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let waiver = guard_waiver(guard_id, waiver_reason);
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{work_id}\"\ntitle = \"Guarded Item\"\nstatus = \"active\"\ncreated = \"2026-01-01\"\nstarted = \"2026-01-01\"\n\n[content]\ndescription = \"Guarded work item\"\n\n[[content.acceptance_criteria]]\ntext = \"done criteria\"\nstatus = \"done\"\ncategory = \"chore\"\n\n[verification]\nrequired_guards = [\"{guard_id}\"]{waiver}"
    );
    write_guarded_work_item_content(dir, &content)
}

fn write_guarded_work_item_content(
    dir: &Path,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join("gov/work/2026-01-01-guarded-item.toml");
    fs::write(path, content)?;
    Ok(())
}

fn guard_waiver(guard_id: &str, waiver_reason: Option<&str>) -> String {
    waiver_reason
        .map(|reason| {
            format!("\n[[verification.waivers]]\nguard = \"{guard_id}\"\nreason = \"{reason}\"\n")
        })
        .unwrap_or_default()
}

pub fn init_project_v1() -> Result<TempDir, Box<dyn std::error::Error>> {
    init_project_at(Some(1))
}
