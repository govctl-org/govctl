---
name: init
description: "Set up govctl in the current project. Installs the binary if missing, initializes governance structure."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: [optional setup scope]
---

# /init - Set Up Govctl

Set up govctl in the current project.

**Outputs:** Initialized governance structure, verified local setup, and a recommended next workflow.

## Critical Rules

1. This is a setup workflow, not an implementation workflow. Do not create work items or change product code here.
2. Ask permission before installing `govctl` with `cargo install govctl`.
3. Prefer local `govctl` if it is already available. Only install when the binary is missing.
4. Never edit governed files directly. Use `govctl init`.
5. If setup succeeds and the user wants it recorded, hand off to `/commit` rather than embedding raw VCS commands here.

## Steps

### 1. Check for govctl binary

```bash
govctl --version
```

If missing, ask permission, then install it:

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

## Next Steps

- Use `/discuss` for design work
- Use `/spec` for governance-only artifact maintenance
- Use `/gov` for implementation-bearing work
- Use `/quick` for trivial non-behavioral cleanup
