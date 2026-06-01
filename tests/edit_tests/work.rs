// ============================================================================
// Work Item Field Edit Tests
// ============================================================================

fn read_work_ids(
    project_dir: &std::path::Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(project_dir.join("gov").join("work"))? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let id = content
            .lines()
            .find_map(|line| line.strip_prefix("id = \"")?.strip_suffix('"'))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("missing work item id in {}", path.display()),
                )
            })?;
        ids.push(id.to_string());
    }
    ids.sort();
    Ok(ids)
}

fn set_work_item_id_strategy(project_dir: &std::path::Path, strategy: &str) -> common::TestResult {
    let config_path = project_dir.join("gov").join("config.toml");
    let mut content = std::fs::read_to_string(&config_path)?;
    content.push_str(&format!("\n[work_item]\nid_strategy = \"{strategy}\"\n"));
    std::fs::write(config_path, content)?;
    Ok(())
}

include!("work_tests/fields.rs");
include!("work_tests/acceptance.rs");
include!("work_tests/journal.rs");
include!("work_tests/identity.rs");
include!("work_tests/references.rs");
