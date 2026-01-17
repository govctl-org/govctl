fn main() {
    // Recompile if embedded assets change
    println!("cargo:rerun-if-changed=assets/gov.md");
    println!("cargo:rerun-if-changed=assets/quick.md");
    println!("cargo:rerun-if-changed=assets/status.md");
}
