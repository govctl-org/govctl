use std::fs;
use std::path::Path;

use super::command;

pub fn loop_id(date: &str, sequence: u32) -> String {
    format!("LOOP-{date}-{sequence:03}")
}

pub fn loop_start(work_ids: &[&str]) -> Vec<String> {
    let mut cmd = command(&["loop", "start"]);
    cmd.extend(work_ids.iter().map(|work_id| (*work_id).to_string()));
    cmd
}

pub fn loop_start_with_id(loop_id: &str, work_ids: &[&str]) -> Vec<String> {
    let mut cmd = command(&["loop", "start", "--id", loop_id]);
    cmd.extend(work_ids.iter().map(|work_id| (*work_id).to_string()));
    cmd
}

pub fn loop_start_with_id_dry_run(loop_id: &str, work_ids: &[&str]) -> Vec<String> {
    let mut cmd = loop_start_with_id(loop_id, work_ids);
    cmd.push("--dry-run".to_string());
    cmd
}

pub fn loop_show(loop_id: &str) -> Vec<String> {
    command(&["loop", "show", loop_id])
}

pub fn loop_resume(loop_id: &str) -> Vec<String> {
    command(&["loop", "resume", loop_id])
}

pub fn loop_add_work(loop_id: &str, work_id: &str) -> Vec<String> {
    loop_add_field(loop_id, "work", work_id)
}

pub fn loop_add_wi(loop_id: &str, work_id: &str) -> Vec<String> {
    loop_add_field(loop_id, "wi", work_id)
}

pub fn loop_add_field(loop_id: &str, field: &str, work_id: &str) -> Vec<String> {
    command(&["loop", "add", loop_id, field, work_id])
}

pub fn loop_remove_work(loop_id: &str, work_id: &str) -> Vec<String> {
    loop_remove_field(loop_id, "work", work_id)
}

pub fn loop_remove_wi(loop_id: &str, work_id: &str) -> Vec<String> {
    loop_remove_field(loop_id, "wi", work_id)
}

pub fn loop_remove_field(loop_id: &str, field: &str, work_id: &str) -> Vec<String> {
    command(&["loop", "remove", loop_id, field, work_id])
}

pub fn loop_replan(loop_id: &str) -> Vec<String> {
    command(&["loop", "replan", loop_id])
}

pub fn loop_list(args: &[&str]) -> Vec<String> {
    let mut cmd = command(&["loop", "list"]);
    cmd.extend(args.iter().map(|arg| (*arg).to_string()));
    cmd
}

pub fn loop_run(loop_id: &str) -> Vec<String> {
    command(&["loop", "run", loop_id])
}

pub fn loop_run_with_max_rounds(loop_id: &str, max_rounds: &str) -> Vec<String> {
    command(&["loop", "run", loop_id, "--max-rounds", max_rounds])
}

pub fn loop_run_target(loop_id: &str, work_id: &str) -> Vec<String> {
    command(&["loop", "run", loop_id, "--work", work_id])
}

pub fn write_guard(dir: &Path, guard_id: &str, command: &str) -> super::TestResult {
    super::write_guard_with_timeout(dir, guard_id, command, None, 300)
}

pub fn append_required_guard(
    dir: &Path,
    date: &str,
    slug: &str,
    guard_id: &str,
) -> super::TestResult {
    let path = dir.join(format!("gov/work/{date}-{slug}.toml"));
    let mut content = fs::read_to_string(&path)?;
    content.push_str(&format!(
        "\n[verification]\nrequired_guards = [\"{guard_id}\"]\n"
    ));
    fs::write(path, content)?;
    Ok(())
}

pub fn read_round_record(
    dir: &Path,
    loop_id: &str,
    work_id: &str,
    round: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(dir.join(format!(
        ".govctl/loops/{loop_id}/rounds/{work_id}/round-{round:03}.toml"
    )))?)
}

pub fn validate_toml_against_schema(
    dir: &Path,
    schema_filename: &str,
    toml_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let errors = schema_validation_errors(dir, schema_filename, toml_text)?;
    assert!(
        errors.is_empty(),
        "{schema_filename} validation errors: {errors:#?}"
    );
    Ok(())
}

pub fn assert_schema_rejects(
    dir: &Path,
    schema_filename: &str,
    toml_text: &str,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let errors = schema_validation_errors(dir, schema_filename, toml_text)?;
    assert!(!errors.is_empty(), "{context}");
    Ok(())
}

fn schema_validation_errors(
    dir: &Path,
    schema_filename: &str,
    toml_text: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let schema_text = fs::read_to_string(dir.join("gov/schema").join(schema_filename))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;
    let compiled = jsonschema::validator_for(&schema)?;
    let toml_value: toml::Value = toml::from_str(toml_text)?;
    let json_value = serde_json::to_value(toml_value)?;
    Ok(compiled
        .iter_errors(&json_value)
        .map(|err| err.to_string())
        .collect())
}

pub fn toml_string(value: &toml::Value, key: &str) -> Result<String, Box<dyn std::error::Error>> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field: {key}").into())
}

pub fn toml_int(value: &toml::Value, key: &str) -> Result<i64, Box<dyn std::error::Error>> {
    value
        .get(key)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| format!("missing integer field: {key}").into())
}

pub fn loop_item_status(
    state_toml: &str,
    work_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let state: toml::Value = toml::from_str(state_toml)?;
    Ok(loop_item_table(&state, work_id)?
        .get("status")
        .and_then(toml::Value::as_str)
        .ok_or("missing loop item status")?
        .to_string())
}

pub fn loop_item_round_count(
    state_toml: &str,
    work_id: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let state: toml::Value = toml::from_str(state_toml)?;
    loop_item_table(&state, work_id)?
        .get("round_count")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| "missing loop item round_count".into())
}

pub fn loop_work(value: &toml::Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    toml_string_array(value, &["loop", "work"])
}

pub fn loop_resolved(value: &toml::Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    toml_string_array(value, &["loop", "resolved"])
}

fn toml_string_array(
    value: &toml::Value,
    path: &[&str],
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut current = value;
    for segment in path {
        current = current
            .get(*segment)
            .ok_or_else(|| format!("missing TOML segment: {segment}"))?;
    }
    current
        .as_array()
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            format!("missing array at path: {}", path.join(".")).into()
        })?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("non-string value at path: {}", path.join(".")).into())
        })
        .collect()
}

pub fn loop_item_table<'a>(
    state: &'a toml::Value,
    work_id: &str,
) -> Result<&'a toml::value::Table, Box<dyn std::error::Error>> {
    state
        .get("items")
        .and_then(toml::Value::as_table)
        .and_then(|items| items.get(work_id))
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing loop item table for {work_id}").into())
}
