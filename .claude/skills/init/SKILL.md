---
name: init
description: "Set up govctl in the current project with explicit installation and overwrite authorization"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional setup scope]"
---

# Initialize Govctl

Establish a valid local governance scaffold and report the workflow that should
own the user's next task. Setup does not create product behavior or governance
history.

## Discovery

Check for `gov/config.toml`, a usable local `govctl`, and any existing
project-local agent assets. In the govctl source repository, prefer
`cargo run --quiet --` as the development invocation. Elsewhere, use the
available project-local or installed binary.

Use `govctl init --help`, `govctl agent --help`, and
`govctl init-skills --help` for current options, destination resolution, and
overwrite behavior. Use `agent doctor`, `agent install`, and `agent update` for
user-scoped runtime integration. Reserve `init-skills` for project-local or
custom-directory projection.
Before `init-skills`, map the logical destination to its resolved target,
inspect existing path ancestors for symlinks, and determine whether the target
is contained by the project root.

## Hard Stops

- Do not create Work Items, product code, RFCs, or ADRs during setup.
- Obtain explicit user authorization before installing a binary or using any
  `--force` option.
- Obtain explicit user authorization before `agent install` or `agent update`
  changes user-scoped runtime configuration.
- Do not run `init --force` as a generic repair for an existing project.
- Do not overwrite project-local skills or agents without inspecting their
  destination and confirming that replacement is intended.
- Obtain separate explicit authorization before `init-skills` creates or
  replaces files when the resolved destination is outside the project root or
  any existing ancestor is a symlink. Show the exact resolved target first.
- Do not edit governed files directly; let `govctl init` own the scaffold.
- Hand raw VCS work to `commit`.

## Setup Policy

If no usable invocation exists, report the missing prerequisite. Ask before
installing `govctl`; if Rust tooling is also absent, stop with the required
installation dependency rather than modifying the project.

When `gov/config.toml` is absent, initialize through `govctl init`. When it is
present, treat the project as initialized and inspect `govctl status` rather
than running initialization again. Follow diagnostics: outdated schemas or
project-support files belong to deterministic `govctl migrate`, while invalid
artifacts require correction through their owning workflows.

For user-scoped integration, run `agent doctor` before the authorized
`agent install` or `agent update`, report failures without substituting manual
runtime commands, and tell the user to start a new session after success. The
installed session hook stays silent outside governed projects. Its direct-edit
guidance is advisory: prefer the canonical CLI when it can express the change,
but use direct editing for unsupported operations and run `govctl check`
afterward.

Project-local skills and reviewer agents are optional. Project them only when
the user wants local copies, using `init-skills` destination and format
discovery. Default to a non-symlinked destination within the project. Preserve
existing assets unless their replacement was explicitly authorized.

## Completion Evidence

Setup is complete when:

- `gov/config.toml` and the expected scaffold exist;
- `govctl status` can read the project;
- initialization or asset-install diagnostics have no unresolved failure;
- the report distinguishes files created, skipped, and intentionally replaced;
  and
- the user is directed to `discuss`, `spec`, `gov`, `quick`, or `migrate`
  according to the next task.
