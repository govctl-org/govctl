//! Tests for agent_dir configuration and init-skills command. [[ADR-0035]]

mod common;

use common::{init_project, run_commands};
use std::fs;

#[test]
fn test_default_agent_dir() -> common::TestResult {
    let temp_dir = init_project()?;

    let _output = run_commands(temp_dir.path(), &[&["init-skills"]])?;

    let skill_dir = temp_dir.path().join(".claude/skills/gov/SKILL.md");
    assert!(
        skill_dir.exists(),
        "skills/gov/SKILL.md should exist under .claude"
    );

    let rfc_writer = temp_dir.path().join(".claude/skills/rfc-writer/SKILL.md");
    assert!(
        rfc_writer.exists(),
        "skills/rfc-writer/SKILL.md should exist under .claude"
    );
    Ok(())
}

#[test]
fn test_init_skills_excludes_plugin_only_init_skill() -> common::TestResult {
    let temp_dir = init_project()?;

    run_commands(temp_dir.path(), &[&["init-skills"]])?;

    let init_dir = temp_dir.path().join(".claude/skills/init");
    assert!(
        !init_dir.exists(),
        "init is a plugin/global onboarding skill, not a project-local init-skills asset"
    );
    let init_skill = temp_dir.path().join(".claude/skills/init/SKILL.md");
    assert!(
        !init_skill.exists(),
        "init is a plugin/global onboarding skill, not a project-local init-skills asset"
    );
    Ok(())
}

#[test]
fn test_init_skills_dry_run_skips_existing_assets_without_force() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(temp_dir.path(), &[&["init-skills"]])?;
    let skill_path = temp_dir.path().join(".claude/skills/gov/SKILL.md");
    let before = fs::read(&skill_path)?;

    let output = run_commands(temp_dir.path(), &[&["init-skills", "--dry-run"]])?;

    assert!(output.contains("exit: 0"), "{output}");
    assert!(!output.contains("Would write"), "{output}");
    assert_eq!(fs::read(&skill_path)?, before);
    Ok(())
}

#[test]
fn test_custom_agent_dir() -> common::TestResult {
    let temp_dir = init_project()?;

    let config_path = temp_dir.path().join("gov/config.toml");
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(&config_path)?)?;
    config["paths"]
        .as_table_mut()
        .ok_or("missing paths table")?
        .insert(
            "agent_dir".to_string(),
            toml::Value::String(".custom-agent".to_string()),
        );
    fs::write(&config_path, toml::to_string_pretty(&config)?)?;

    run_commands(temp_dir.path(), &[&["init-skills", "-f"]])?;

    let cursor_skill = temp_dir.path().join(".custom-agent/skills/gov/SKILL.md");
    assert!(
        cursor_skill.exists(),
        "skills/gov/SKILL.md should exist under custom agent_dir, found: {:?}",
        cursor_skill
    );
    Ok(())
}

#[test]
fn test_agent_dir_creates_subdirs() -> common::TestResult {
    let temp_dir = init_project()?;

    run_commands(temp_dir.path(), &[&["init-skills"]])?;

    assert!(temp_dir.path().join(".claude/skills").is_dir());
    assert!(temp_dir.path().join(".claude/agents").is_dir());
    assert!(!temp_dir.path().join(".claude/commands").exists());
    Ok(())
}

#[test]
fn test_codex_format_agents() -> common::TestResult {
    let temp_dir = init_project()?;

    run_commands(temp_dir.path(), &[&["init-skills", "--format", "codex"]])?;

    assert!(temp_dir.path().join(".codex/skills/gov/SKILL.md").exists());

    let toml_agent = temp_dir.path().join(".codex/agents/rfc-reviewer.toml");
    assert!(
        toml_agent.exists(),
        "codex format should write .toml agents"
    );
    let content = fs::read_to_string(&toml_agent)?;
    let agent: toml::Value = toml::from_str(&content)?;
    assert_eq!(agent["name"].as_str(), Some("rfc-reviewer"));
    assert!(agent["description"].as_str().is_some());
    assert!(agent["developer_instructions"].as_str().is_some());
    assert_eq!(agent["sandbox_mode"].as_str(), Some("read-only"));

    assert!(
        !temp_dir
            .path()
            .join(".codex/agents/rfc-reviewer.md")
            .exists(),
        "codex format should not write .md agents"
    );
    assert!(
        !temp_dir.path().join(".claude").exists(),
        "format-implied Codex output should not use the Claude directory"
    );
    Ok(())
}

#[test]
fn test_claude_format_agents() -> common::TestResult {
    let temp_dir = init_project()?;

    run_commands(temp_dir.path(), &[&["init-skills"]])?;

    let md_agent = temp_dir.path().join(".claude/agents/rfc-reviewer.md");
    assert!(md_agent.exists(), "claude format should write .md agents");

    assert!(
        !temp_dir
            .path()
            .join(".claude/agents/rfc-reviewer.toml")
            .exists(),
        "claude format should not write .toml agents"
    );
    Ok(())
}

#[test]
fn test_init_no_skills() -> common::TestResult {
    let temp_dir = init_project()?;

    // Skills and agents are intentionally created only by init-skills. [[ADR-0035]]
    assert!(
        !temp_dir.path().join(".claude/skills").exists(),
        "init should not create .claude/skills"
    );
    assert!(
        !temp_dir.path().join(".claude/agents").exists(),
        "init should not create .claude/agents"
    );
    assert!(
        temp_dir.path().join("gov/schema/adr.schema.json").exists(),
        "init should create schema files"
    );
    Ok(())
}
