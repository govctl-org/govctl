use clap::ValueEnum;

/// Output format for agent definitions in `init-skills`.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum SkillFormat {
    /// Claude Code / Cursor / Windsurf (agents as .md with YAML frontmatter)
    #[default]
    Claude,
    /// Codex CLI (agents as .toml with developer_instructions)
    Codex,
}
