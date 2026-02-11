fn main() {
    // Recompile if embedded command templates change
    println!("cargo:rerun-if-changed=.claude/commands/gov.md");
    println!("cargo:rerun-if-changed=.claude/commands/quick.md");
    println!("cargo:rerun-if-changed=.claude/commands/status.md");
    println!("cargo:rerun-if-changed=.claude/commands/discuss.md");
}
