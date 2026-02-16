//! Tests for agent_dir configuration (rename from commands_dir).

mod common;

use common::{init_project, run_commands};
use std::fs;

/// Test: Default agent_dir is .claude
#[test]
fn test_default_agent_dir() {
    let temp_dir = init_project();

    // sync-commands should create files under .claude/commands by default
    let _output = run_commands(temp_dir.path(), &[&["sync-commands"]]);

    // Check that .claude/commands/gov.md exists
    let commands_dir = temp_dir.path().join(".claude/commands/gov.md");
    assert!(
        commands_dir.exists(),
        "commands/gov.md should exist under .claude"
    );

    // Check that skills exist
    let skill_dir = temp_dir.path().join(".claude/skills/rfc-writer/SKILL.md");
    assert!(
        skill_dir.exists(),
        "skills/rfc-writer/SKILL.md should exist under .claude"
    );
}

/// Test: Custom agent_dir is respected
#[test]
fn test_custom_agent_dir() {
    let temp_dir = init_project();

    // Update config to use .cursor instead of .claude
    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
gov_root = "gov"
docs_output = "docs"
agent_dir = ".cursor"
"#;
    fs::write(&config_path, config_content).unwrap();

    // sync-commands should create files under .cursor/commands
    let output = run_commands(temp_dir.path(), &[&["sync-commands", "-f"]]);
    eprintln!("sync-commands output:\n{}", output);

    // List created directories
    if let Ok(entries) = fs::read_dir(temp_dir.path()) {
        for entry in entries.flatten() {
            eprintln!("  {:?}", entry.path());
        }
    }

    // Check that .cursor/commands/gov.md exists
    let cursor_commands = temp_dir.path().join(".cursor/commands/gov.md");
    assert!(
        cursor_commands.exists(),
        "commands/gov.md should exist under .cursor, found: {:?}",
        cursor_commands
    );
}

/// Test: agent_dir creates all subdirs (commands, skills, agents)
#[test]
fn test_agent_dir_creates_subdirs() {
    let temp_dir = init_project();

    // sync-commands should create all subdirs
    run_commands(temp_dir.path(), &[&["sync-commands"]]);

    // Verify all expected subdirs exist
    assert!(temp_dir.path().join(".claude/commands").is_dir());
    assert!(temp_dir.path().join(".claude/skills").is_dir());
    assert!(temp_dir.path().join(".claude/agents").is_dir());
}
