fn main() {
    // Recompile if embedded assets change
    println!("cargo:rerun-if-changed=assets/do.md");
}
