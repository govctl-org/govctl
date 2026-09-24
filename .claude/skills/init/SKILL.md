---
name: init
description: "Set up govctl in the current project with explicit installation and overwrite authorization"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional setup scope]"
---

# Initialize Govctl

Create a valid governance scaffold and point the user to the right workflow for
their next task. Setup creates no product behavior or governance history.

## Discovery

Check for `gov/config.toml`, a usable `govctl`, and existing local agent assets.
Use `govctl init --help`, `govctl agent --help`, and
`govctl init-skills --help` for current options and overwrite behavior. `agent doctor|install|update` manage user-scoped runtime
integration; `init-skills` writes project-local or custom-directory copies.

## Hard Stops

- Create no Work Items, code, RFCs, or ADRs.
- Get explicit approval before installing a binary, using any `--force`, or
  running `agent install` or `agent update`.
- Never use `init --force` to repair an existing project.
- Before `init-skills`, resolve the destination, check its existing ancestors
  for symlinks, and check whether it stays inside the project root. Confirm any
  replacement of existing skills or agents.
- If the resolved target is outside the project root or behind a symlinked
  ancestor, show it and get separate approval.
- Do not edit governed files directly; `govctl init` owns the scaffold.
- Hand VCS work to `commit`.

## Setup

If `govctl` is missing, ask before installing it; if Rust tooling is also
missing, stop and name the dependency.

Run `govctl init` only when `gov/config.toml` is absent; otherwise inspect
`govctl status`. Outdated schemas go to `govctl migrate`; invalid artifacts go
to their owning workflows.

For runtime integration, run `agent doctor` first, report failures instead of
substituting manual commands, and tell the user to start a new session after
success. The installed hook is silent outside governed projects, and its
direct-edit guidance is advisory: use the CLI when it supports a change,
otherwise edit directly and run `govctl check`.

Local skills and agents are optional; install them only on request, default to
a non-symlinked destination inside the project, and keep existing assets
unless replacement was approved.

## Done When

- the scaffold exists and `govctl status` reads it;
- diagnostics show no unresolved failure;
- the report separates files created, skipped, and replaced; and
- the user is pointed to `discuss`, `spec`, `gov`, `quick`, or `migrate`.
