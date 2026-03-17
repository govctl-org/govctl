//! Tests for agent_dir configuration and init-skills command. [[ADR-0035]]

mod common;

use common::{init_project, run_commands};
use std::fs;

/// Test: Default agent_dir is .claude
#[test]
fn test_default_agent_dir() {
    let temp_dir = init_project();

    let _output = run_commands(temp_dir.path(), &[&["init-skills"]]);

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
}

/// Test: Custom agent_dir is respected
#[test]
fn test_custom_agent_dir() {
    let temp_dir = init_project();

    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
docs_output = "docs"
agent_dir = ".custom-agent"
"#;
    fs::write(&config_path, config_content).unwrap();

    let output = run_commands(temp_dir.path(), &[&["init-skills", "-f"]]);
    eprintln!("init-skills output:\n{}", output);

    if let Ok(entries) = fs::read_dir(temp_dir.path()) {
        for entry in entries.flatten() {
            eprintln!("  {:?}", entry.path());
        }
    }

    let cursor_skill = temp_dir.path().join(".custom-agent/skills/gov/SKILL.md");
    assert!(
        cursor_skill.exists(),
        "skills/gov/SKILL.md should exist under custom agent_dir, found: {:?}",
        cursor_skill
    );
}

/// Test: init-skills creates all subdirs (skills, agents)
#[test]
fn test_agent_dir_creates_subdirs() {
    let temp_dir = init_project();

    run_commands(temp_dir.path(), &[&["init-skills"]]);

    assert!(temp_dir.path().join(".claude/skills").is_dir());
    assert!(temp_dir.path().join(".claude/agents").is_dir());
    assert!(!temp_dir.path().join(".claude/commands").exists());
}

/// Test: init does NOT create skills/agents [[ADR-0035]]
#[test]
fn test_init_no_skills() {
    let temp_dir = init_project();

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
}
