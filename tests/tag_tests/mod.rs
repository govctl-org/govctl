use crate::common::{TestResult, init_project, normalize_output, run_commands, today};
use std::fs;

mod artifacts;
mod filtering;
mod registry;

fn assert_tag_snapshot(name: &str, value: String) {
    let snapshot_name = crate::common::named_snapshot_name("test_tags", name);
    crate::with_test_snapshot_settings!({
        insta::assert_snapshot!(snapshot_name, value);
    });
}

fn register_tags(dir: &std::path::Path, tags: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let content = fs::read_to_string(&config_path)?;
    let mut doc: toml::Table = toml::from_str(&content)?;
    let arr: toml::value::Array = tags
        .iter()
        .map(|t| toml::Value::String(t.to_string()))
        .collect();
    let mut tags_table = toml::Table::new();
    tags_table.insert("allowed".into(), toml::Value::Array(arr));
    doc.insert("tags".into(), toml::Value::Table(tags_table));
    fs::write(&config_path, toml::to_string_pretty(&doc)?)?;
    Ok(())
}
