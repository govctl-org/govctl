---
name: init
description: "Set up govctl in the current project. Installs the binary if missing, initializes governance structure."
---

# govctl Init

Set up govctl in the current project.

## Steps

### 1. Check for govctl binary

```bash
govctl --version
```

If missing, install it:

```bash
cargo install govctl
```

If `cargo` is also missing, tell the user to install Rust first: https://rustup.rs

### 2. Initialize the project

If `gov/config.toml` does not exist:

```bash
govctl init
```

If it already exists, skip — the project is already initialized.

### 3. Verify

```bash
govctl status
```

Show the user what was created and confirm everything is working.
