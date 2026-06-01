use std::fs;
use std::path::Path;

pub fn loop_id(date: &str, sequence: u32) -> String {
    format!("LOOP-{date}-{sequence:03}")
}

pub fn write_guard(dir: &Path, guard_id: &str, command: &str) -> super::TestResult {
    let path = dir
        .join("gov/guard")
        .join(format!("{}.toml", guard_id.to_lowercase()));
    let content = format!(
        "[govctl]\nschema = 1\nid = \"{guard_id}\"\ntitle = \"{guard_id}\"\n\n[check]\ncommand = \"{command}\"\ntimeout_secs = 300\n"
    );
    fs::write(path, content)?;
    Ok(())
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
    let schema_text = fs::read_to_string(dir.join("gov/schema").join(schema_filename))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;
    let compiled = jsonschema::validator_for(&schema)?;
    let toml_value: toml::Value = toml::from_str(toml_text)?;
    let json_value = serde_json::to_value(toml_value)?;
    let errors = compiled
        .iter_errors(&json_value)
        .map(|err| err.to_string())
        .collect::<Vec<_>>();
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
    let schema_text = fs::read_to_string(dir.join("gov/schema").join(schema_filename))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;
    let compiled = jsonschema::validator_for(&schema)?;
    let toml_value: toml::Value = toml::from_str(toml_text)?;
    let json_value = serde_json::to_value(toml_value)?;
    let errors = compiled
        .iter_errors(&json_value)
        .map(|err| err.to_string())
        .collect::<Vec<_>>();
    assert!(!errors.is_empty(), "{context}");
    Ok(())
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

pub fn loop_roots(value: &toml::Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    toml_string_array(value, &["loop", "root_work_items"])
}

pub fn loop_work_items(value: &toml::Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    toml_string_array(value, &["loop", "work_items"])
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
