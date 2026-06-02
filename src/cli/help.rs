pub(super) const INIT: &str = r#"EXAMPLES:
    govctl init
    govctl init --force

NOTES:
    - Creates `gov/`, `gov/config.toml`, and baseline governance artifacts.
    - Use `--force` to overwrite an existing initialization.
"#;

pub(super) const INIT_SKILLS: &str = r#"EXAMPLES:
    govctl init-skills
    govctl init-skills --force

NOTES:
    - Installs or refreshes project-local skills and agents.
    - Use `--force` to overwrite existing generated assets.
"#;

pub(super) const CHECK: &str = r#"EXAMPLES:
    govctl check
    govctl check -W
    govctl check --has-active

NOTES:
    - `-W/--deny-warnings` treats warnings as errors.
    - `--has-active` asserts that an active work item exists.
"#;

pub(super) const STATUS: &str = r#"EXAMPLES:
    govctl status

NOTES:
    - Prints high-level counts for governed artifacts.
"#;

pub(super) const RENDER: &str = r#"EXAMPLES:
    govctl render
    govctl render adr
    govctl render work --dry-run
    govctl render changelog --force

NOTES:
    - This is a bulk render entrypoint.
    - For a single artifact, use resource render:
      `govctl rfc render <ID>`, `govctl adr render <ID>`, `govctl work render <ID>`.
"#;

pub(super) const MIGRATE: &str = r#"EXAMPLES:
    govctl migrate
    govctl --dry-run migrate

NOTES:
    - Reads legacy JSON artifacts and upgrades them to current canonical storage.
    - Intended for one-time repository migration, not normal day-to-day editing.
"#;

pub(super) const VERIFY: &str = r#"EXAMPLES:
    govctl verify GUARD-CLIPPY
    govctl verify GUARD-CLIPPY GUARD-TESTS
    govctl verify --work WI-2026-04-06-001

NOTES:
    - Pass guard IDs to run specific guards directly.
    - Use `--work` to run the effective guard set for a work item.
    - `--work` conflicts with explicit guard IDs.
"#;

pub(super) const LOOP: &str = r#"COMMON WORKFLOW:
    1. `govctl loop list open` to discover existing non-terminal loops
    2. `govctl loop start WI-2026-04-06-001` to create local loop state
    3. `govctl loop run <LOOP-ID>` to execute one round for ready work
    4. `govctl loop show <LOOP-ID>` to inspect persisted state
    5. `govctl loop resume <LOOP-ID>` to resume discovered loop state
    6. `govctl loop add <LOOP-ID> work WI-2026-04-06-002` to expand scope

NOTES:
    - Loop state is local under `.govctl/loops/<LOOP-ID>/state.toml`.
    - Use `loop list open` before guessing a loop ID or work set after interruption.
    - Use `loop run <LOOP-ID> --work <WI-ID>` to target work inside a loop.
    - `loop run` uses work-item lifecycle commands for status transitions.
"#;

pub(super) const RFC: &str = r#"COMMON WORKFLOW:
    1. `govctl rfc list` to discover RFCs
    2. `govctl rfc get <ID> ...` for metadata/fields
    3. `govctl rfc show <ID>` for rendered prose
    4. `govctl rfc edit <ID> ...` to update content
    5. `govctl rfc finalize/advance/...` for lifecycle

START HERE:
    - New RFC: `govctl rfc new "Title"`
    - Inspect one RFC: `govctl rfc get RFC-0001`
    - Render one RFC: `govctl rfc show RFC-0001`
"#;

pub(super) const CLAUSE: &str = r#"COMMON WORKFLOW:
    1. `govctl clause list` to discover clauses
    2. `govctl clause get <ID> ...` for metadata/fields
    3. `govctl clause show <ID>` for rendered clause text
    4. `govctl clause edit <ID> ...` to update content
    5. `govctl clause deprecate/supersede` for lifecycle

START HERE:
    - New clause: `govctl clause new RFC-0001:C-SCOPE "Scope"`
    - Inspect one clause: `govctl clause get RFC-0001:C-SCOPE`
"#;

pub(super) const ADR: &str = r#"COMMON WORKFLOW:
    1. `govctl adr list` to discover ADRs
    2. `govctl adr get <ID> ...` for metadata/fields
    3. `govctl adr show <ID>` for rendered prose
    4. `govctl adr edit/add/tick` to work through alternatives
    5. `govctl adr accept/reject/...` for lifecycle

START HERE:
    - New ADR: `govctl adr new "Title"`
    - Inspect one ADR: `govctl adr get ADR-0001`
    - Move an alternative to accepted: `govctl adr tick ADR-0001 alternatives --at 0 -s accepted`
"#;

pub(super) const WORK: &str = r#"COMMON WORKFLOW:
    1. `govctl work list` to discover work items
    2. `govctl work get <ID> ...` for metadata/fields
    3. `govctl work edit/add` to define scope and acceptance criteria
    4. `govctl work tick` to update acceptance-criteria status
    5. `govctl work move` to change lifecycle state

START HERE:
    - New work item: `govctl work new "Title"`
    - Activate work: `govctl work move WI-<DATE>-001 active`
    - Inspect one work item: `govctl work get WI-<DATE>-001`
"#;

pub(super) const GUARD: &str = r#"COMMON WORKFLOW:
    1. `govctl guard list` to discover guards
    2. `govctl guard get <ID> ...` for metadata/fields
    3. `govctl guard edit/set` to define checks
    4. `govctl verify <GUARD-ID>` or `govctl verify --work <WI-ID>` to run guards

START HERE:
    - New guard: `govctl guard new "clippy lint"`
    - Inspect one guard: `govctl guard get GUARD-CLIPPY`
"#;

pub(super) const RELEASE: &str = r#"EXAMPLES:
    govctl release 0.2.0
    govctl release 0.2.0 --date 2026-04-07

NOTES:
    - Collects unreleased completed work items into a versioned release.
    - Use a semver version string.
"#;

pub(super) const DESCRIBE: &str = r#"EXAMPLES:
    govctl describe
    govctl describe --context
    govctl describe -o json

NOTES:
    - `--context` includes current project state and suggested next actions.
    - Output is intended for agents and tooling.
"#;

pub(super) const COMPLETIONS: &str = r#"EXAMPLES:
    govctl completions bash
    govctl completions zsh

NOTES:
    - Writes completion script text to stdout for the selected shell.
"#;

pub(super) const SELF_UPDATE: &str = r#"EXAMPLES:
    govctl self-update
    govctl self-update --check

NOTES:
    - Downloads the latest binary from GitHub Releases and replaces the current executable.
    - Use `--check` to see if an update is available without installing it.
    - Implements [[RFC-0002:C-SELF-UPDATE]].
"#;

pub(super) const TAG: &str = r#"EXAMPLES:
    govctl tag list
    govctl tag new caching
    govctl tag delete caching

NOTES:
    - Tags are defined project-wide in gov/config.toml [tags] allowed.
    - Artifacts may only reference tags declared here.
    - Implements [[RFC-0002:C-RESOURCES]] controlled-vocabulary tags.
"#;
