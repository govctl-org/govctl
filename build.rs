fn main() {
    // Recompile if any embedded .claude/ assets change
    // Commands
    println!("cargo:rerun-if-changed=.claude/commands/gov.md");
    println!("cargo:rerun-if-changed=.claude/commands/quick.md");
    println!("cargo:rerun-if-changed=.claude/commands/status.md");
    println!("cargo:rerun-if-changed=.claude/commands/discuss.md");
    // Skills
    println!("cargo:rerun-if-changed=.claude/skills/rfc-writer/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/adr-writer/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/wi-writer/SKILL.md");
    // Agents
    println!("cargo:rerun-if-changed=.claude/agents/rfc-reviewer.md");
    println!("cargo:rerun-if-changed=.claude/agents/adr-reviewer.md");
    println!("cargo:rerun-if-changed=.claude/agents/wi-reviewer.md");
    println!("cargo:rerun-if-changed=.claude/agents/compliance-checker.md");
}
