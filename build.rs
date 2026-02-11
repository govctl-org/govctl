fn main() {
    // Recompile if embedded assets change
    println!("cargo:rerun-if-changed=assets/commands/gov.md");
    println!("cargo:rerun-if-changed=assets/commands/quick.md");
    println!("cargo:rerun-if-changed=assets/commands/status.md");
    println!("cargo:rerun-if-changed=assets/commands/discuss.md");
}
