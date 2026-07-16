use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Get today's date in YYYY-MM-DD format (same as govctl uses)
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
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

pub fn temp_dir_with_date() -> Result<(TempDir, String), std::io::Error> {
    let temp_dir = TempDir::new()?;
    let date = today();
    Ok((temp_dir, date))
}

pub fn init_project_with_date() -> Result<(TempDir, String), Box<dyn std::error::Error>> {
    let temp_dir = init_project()?;
    let date = today();
    Ok((temp_dir, date))
}

pub fn work_id(date: &str, sequence: u32) -> String {
    format!("WI-{date}-{sequence:03}")
}

pub fn first_work_id(date: &str) -> String {
    work_id(date, 1)
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

pub fn write_minimal_rfc(
    dir: &Path,
    rfc_id: &str,
    title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let rfc_dir = dir.join("gov/rfc").join(rfc_id);
    fs::create_dir_all(rfc_dir.join("clauses"))?;
    let content = format!(
        r#"[govctl]
schema = 1
id = "{rfc_id}"
title = "{title}"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test-user"]
created = "2026-01-01"

[[sections]]
title = "Specification"

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "Initial version"
"#
    );
    fs::write(rfc_dir.join("rfc.toml"), content)?;
    Ok(())
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
