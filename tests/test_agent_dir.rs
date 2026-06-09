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
fn test_wi_writer_recommends_verification_guards() -> common::TestResult {
    let temp_dir = init_project()?;

    run_commands(temp_dir.path(), &[&["init-skills"]])?;

    let wi_writer = temp_dir.path().join(".claude/skills/wi-writer/SKILL.md");
    let content = fs::read_to_string(&wi_writer)?;
    assert!(
        content.contains("Guardable Command Checks"),
        "wi-writer should include guard guidance for command-style checks"
    );
    assert!(
        content.contains("verification.required_guards"),
        "wi-writer should mention per-work-item verification guards"
    );
    assert!(
        content.contains("verification.default_guards"),
        "wi-writer should mention project-level default guards"
    );
    assert!(
        content.contains("Project-level `verification.default_guards` apply broadly"),
        "wi-writer should explain how default guards relate to per-work-item guards"
    );
    Ok(())
}

#[test]
fn test_custom_agent_dir() -> common::TestResult {
    let temp_dir = init_project()?;

    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
docs_output = "docs"
agent_dir = ".custom-agent"
"#;
    fs::write(&config_path, config_content)?;

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

    assert!(temp_dir.path().join(".claude/skills/gov/SKILL.md").exists());

    let toml_agent = temp_dir.path().join(".claude/agents/rfc-reviewer.toml");
    assert!(
        toml_agent.exists(),
        "codex format should write .toml agents"
    );
    let content = fs::read_to_string(&toml_agent)?;
    assert!(content.contains("name = \"rfc-reviewer\""));
    assert!(content.contains("developer_instructions"));

    assert!(
        !temp_dir
            .path()
            .join(".claude/agents/rfc-reviewer.md")
            .exists(),
        "codex format should not write .md agents"
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
