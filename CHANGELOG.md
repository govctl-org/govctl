# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Release entries are curated summaries for readers. Work item traceability remains in
`gov/releases.toml`.

## [Unreleased]

### Added

- govctl check reports work items that contain legacy inline execution-history entries with an informational diagnostic (WI-2026-05-31-004)
- The diagnostic message points users to notes for durable takeaways and loop state for new execution trace (WI-2026-05-31-004)
- Work item TOML model and schema support depends_on as a list of work item IDs (WI-2026-05-31-008)
- Work item show and render output display dependency declarations distinctly from refs (WI-2026-05-31-008)
- Edit and validation paths reject malformed dependency IDs and unknown dependency targets (WI-2026-05-31-008)
- govctl check and work add reject cyclic work item dependencies (WI-2026-05-31-008)
- Loop state structs serialize and deserialize state.toml under .govctl/loops/<loop-id> (WI-2026-05-31-009)
- Loop state tracks lifecycle status work item statuses dependency graph and round counts (WI-2026-05-31-009)
- Loop state storage is local under .govctl and does not write execution trace to work item fields (WI-2026-05-31-009)
- Planner builds a dependency graph from explicit loop work items and their depends_on fields (WI-2026-05-31-010)
- Planner rejects cycles and missing dependencies with coded diagnostics (WI-2026-05-31-010)
- Planner computes a deterministic execution order for dependency-satisfied work items (WI-2026-05-31-010)
- Planner marks downstream work items blocked in loop state when dependencies fail or cancel (WI-2026-05-31-010)
- CLI exposes loop start and loop show commands for explicit work item sets (WI-2026-05-31-011)
- Loop start creates or reuses local loop state and reports loop ID status and planned work items (WI-2026-05-31-011)
- Loop show renders persisted loop state including work item statuses dependency order and round counts (WI-2026-05-31-011)
- Loop resume detects existing state for the same work item set or reports a clear diagnostic (WI-2026-05-31-011)
- Loop runner transitions queued work items to active through govctl work move semantics (WI-2026-05-31-012)
- Loop runner evaluates acceptance criteria and required guards before moving work items to done (WI-2026-05-31-012)
- Loop runner records round counts statuses and failure or blocked outcomes in loop state (WI-2026-05-31-012)
- Failed work items remain active while loop state records failure and propagates blocked dependents (WI-2026-05-31-012)
- Loop commands can replan an existing loop after adding or removing root work items (WI-2026-06-01-082)
- Loop start and run generate LOOP-YYYY-MM-DD-NNN IDs and reject non-canonical explicit IDs (WI-2026-06-01-082)
- RFC-0006 defines loop listing semantics for local state discovery (WI-2026-06-01-089)
- RFC-0006 changelog records the loop listing addition (WI-2026-06-01-089)
- `govctl loop list` lists persisted loop states without requiring a loop ID (WI-2026-06-01-090)
- loop list supports table, plain, and json output with stable ordering (WI-2026-06-01-090)
- loop list tests cover empty state, multi-loop output, and invalid canonical state handling (WI-2026-06-01-090)
- loop list accepts a filter argument for lifecycle states plus a resumable/open alias covering non-terminal loop states (WI-2026-06-01-099)
- loop list supports -n/--limit after filtering while preserving deterministic loop ID ordering (WI-2026-06-01-099)
- repository tests fail when `include!` is used under `tests/` for test-suite splitting (WI-2026-06-02-058)

### Changed

- Newly created work items omit content.journal from serialized TOML (WI-2026-05-31-003)
- Work item show/render still renders legacy inline journal entries correctly (WI-2026-05-31-003)
- Existing repositories with legacy inline execution-history entries continue to pass govctl check unless they have unrelated blocking errors (WI-2026-05-31-004)
- RFC-0006 defines dynamic root-scope replan/add/remove semantics with dependency closure recomputation (WI-2026-06-01-081)
- RFC-0006 defines how existing loop item state is preserved or removed during scope mutation (WI-2026-06-01-081)
- RFC-0006 specifies canonical loop IDs as LOOP-YYYY-MM-DD-NNN and rejects plain-text loop IDs (WI-2026-06-01-081)
- Replanning recomputes dependency closure while preserving applicable item statuses and round counts (WI-2026-06-01-082)
- direct command-router invariant failures return Diagnostic errors instead of uncoded anyhow messages (WI-2026-06-01-166)
- file I/O and serialization helpers used by CLI commands map failures to Diagnostic values (WI-2026-06-01-166)
- remaining intentional anyhow boundaries are documented as transport or test-only, not user-facing errors (WI-2026-06-01-166)
- scan command and core execution paths for non-Diagnostic error propagation and classify remaining intentional boundaries (WI-2026-06-01-169)
- convert in-scope anyhow/String/std error returns to Diagnostic or Diagnostics without changing CLI behavior (WI-2026-06-01-169)
- RFC-0002 classifies loop as a project-level local execution-state command namespace (WI-2026-06-01-179)
- RFC-0006 defines loop command-surface semantics without positional argument overloading (WI-2026-06-01-179)
- Loop state storage uses `loop.work` for the editable work set and a separate non-editable resolved dependency closure field (WI-2026-06-01-202)
- Loop add/remove help and behavior prefer `work`, accept `wi`, and reject `work_items` as a field name (WI-2026-06-01-202)
- RFC-0006 requires the loop `wi` field alias for `work` scope mutation (WI-2026-06-01-212)
- Loop tests continue to accept `wi` and reject `work_items`/`root_work_items` (WI-2026-06-01-212)
- integration tests use normal Rust modules instead of `include!` for split test suites (WI-2026-06-02-057)
- existing integration-test coverage and insta snapshot names remain stable after module conversion (WI-2026-06-02-057)
- no test file under `tests/` contains `include!(` after the cleanup (WI-2026-06-02-058)
- Small CLI/tag-only helper modules are merged where they add no reusable boundary (WI-2026-06-03-013)
- Edit target handlers no longer use thin get/remove micro modules (WI-2026-06-03-013)
- Loop execution and validation micro helpers are consolidated without behavior changes (WI-2026-06-03-013)
- RFC-0006 defines loop run as a local execution protocol that records and validates round evidence instead of implementing code or directly completing work items (WI-2026-06-03-014)
- loop state and round schemas model loop-level execution rounds, summaries, verification evidence, and note candidates while preserving existing work, depends_on, guards, and notes ownership (WI-2026-06-03-014)
- loop command behavior reuses start/list/show/resume/run/add/remove/replan without adding a parallel resource CRUD model or new testing system (WI-2026-06-03-014)
- embedded gov and wi-writer skills plus CLI describe/help guidance avoid journal and direct agents to loop state for execution trace (WI-2026-06-03-014)

### Removed

- Journal cannot be fetched, added, edited, ticked, or removed as a path-addressable work item field (WI-2026-05-31-003)
- Legacy loop state keys `root_work_items` and `work_items` are not accepted for the unreleased loop schema (WI-2026-06-01-202)

### Fixed

- commit skill detects governance before running govctl commands (WI-2026-05-31-001)
- global skill copy at ~/.agents/skills/commit matches repo-local copy (WI-2026-05-31-001)
- non-governed repos skip govctl check and work item steps (WI-2026-05-31-001)
- governed repos continue to run full govctl workflow unchanged (WI-2026-05-31-001)
- tick matches category-prefixed pattern on acceptance criteria (WI-2026-05-31-002)
- CLI help and routing distinguish loop work-field discovery, scope mutation, resume, and targeted run selectors (WI-2026-06-01-179)
- migrate config version bump participates in the same transactional file operation set as artifact rewrites (WI-2026-06-01-180)
- migrate rollback removes newly created files and restores overwritten or deleted originals after apply failure (WI-2026-06-01-180)
- rfc supersede marks source RFC superseded and records the replacement RFC (WI-2026-06-01-181)
- rfc supersede rejects missing or invalid replacement RFCs with diagnostics (WI-2026-06-01-181)
- clause delete rejects deletion when another artifact references the clause ID (WI-2026-06-01-182)
- clause delete preserves existing draft-status and RFC section update behavior for unreferenced clauses (WI-2026-06-01-182)

## [0.8.5] - 2026-05-25

### Added

- Added controlled-vocabulary tags across the repository and surfaced them in RFC, ADR, work item, and clause list/detail views.
- Expanded the user guide for tags, guard subcommands, `verify`, `self-update`, `init-skills`, `describe`, release workflow, TUI shortcuts, canonical edits, and `govctl migrate` vs `/migrate`.

### Changed

- Made tag listing and deletion scale better by loading artifacts once and reusing a batch tag-usage map.
- Clarified writer/reviewer guidance around global `verification.default_guards`, per-work-item `verification.required_guards`, waivers, and reusable command-style guards.
- Refreshed project documentation, schemas, templates, embedded skills, agents, and CLI help so they match the current command surface and TOML formats.
- Cleaned up governance references by superseding obsolete ADR links and pointing related artifacts at newer ADRs.

### Fixed

- Fixed guard execution for noisy commands, detached child processes, and CI-safe process-group cleanup by collecting output safely, only signaling isolated guard child process groups, and reporting timeout state more clearly.
- Fixed `self-update` archive extraction so release archives and cargo-binstall resolve binaries under `govctl-v{{ version }}-{{ target }}/{{ bin }}`.
- Rendered acceptance-criterion category labels in `govctl work show`, giving reviewers the same category signal as raw TOML.

## [0.8.4] - 2026-04-15

### Added

- Added write-time and lifecycle gates that prevent incomplete ADRs from being accepted or having decisions edited before alternatives are complete.
- Added `adr accept --force` as an explicit escape hatch for historical backfills.

## [0.8.3] - 2026-04-14

### Added

- Added `govctl self-update`, including `--check` mode.
- Added cargo-binstall metadata for binary installation.

## [0.8.2] - 2026-04-10

### Added

- Added tags to RFC, clause, ADR, work item, and guard schemas.
- Added `govctl tag new/delete/list`, tag validation against the configured vocabulary, and `--tag` filters on artifact list commands.

### Fixed

- Fixed `clause edit <ID> text --stdin` so it works without an explicit `--set`.

## [0.8.1] - 2026-04-08

### Added

- Added Codex agent output to `init-skills --format codex`.
- Added `init-skills --dir` for writing generated assets to a caller-provided directory.

## [0.8.0] - 2026-04-08

### Added

- Added canonical `<resource> edit <ID> <path>` entrypoints with `--set`, `--add`, `--remove`, and `--tick`.
- Routed legacy set/add/remove/tick verbs through the same canonical edit planning path, including nested object and array paths.

### Fixed

- Fixed converted-path diagnostics to use stable diagnostic codes.
- Fixed direct CLI command paths so expected user-facing failures no longer surface uncoded temporary errors.

## [0.7.7] - 2026-04-04

### Changed

- Bundled and stamped plugin manifest metadata during release packaging.
- Clarified RFC, ADR, and work-item authority boundaries across skills, reviewer prompts, and guides.

## [0.7.6] - 2026-04-04

### Fixed

- Removed stale `commands/` references from `agent_dir` comments and config templates.

## [0.7.5] - 2026-04-02

### Changed

- Aligned skill `allowed-tools` metadata with Claude Code tool names.
- Reconciled workflow/reviewer handoffs across skills and agents.
- Added a spec-only governance workflow for artifact maintenance.

## [0.7.4] - 2026-03-28

### Added

- Added `rfc add` and `rfc remove` for array fields such as refs, owners, and sections.

## [0.7.3] - 2026-03-28

### Fixed

- Fixed E0112/E0306 diagnostic text so it explains dependency direction instead of bracket syntax.

## [0.7.2] - 2026-03-27

### Added

- Added guard management commands: `govctl guard new`, `list`, `show`, `set`, and `delete`.

## [0.7.1] - 2026-03-26

### Added

- Added `govctl check` enforcement for artifact reference hierarchy and inline `[[...]]` references.

## [0.7.0] - 2026-03-17

### Changed

- Changed `govctl init` to create project files only, with guidance to use `init-skills` or the plugin for skills and agents.
- Renamed the old `sync` command to `init-skills`.

### Fixed

- Fixed migration so bundled schema JSON files are overwritten with the current versions.

## [0.6.0] - 2026-03-17

### Added

- Added runtime JSON Schema validation for RFC, clause, ADR, work item, guard, and release artifacts.
- Added TOML migration for legacy JSON and old-format TOML repositories, including deterministic `#:schema` headers and schema-version upgrades.
- Added reusable verification guard artifacts, default guard enforcement, per-work-item required guards, and structured waivers.
- Added a staged migration engine with backup, commit, and rollback file operations.
- Added semantic terminal colors, styled markdown rendering, and markdown-backed TUI detail views.
- Added TUI scrolling improvements, including half-page and full-page navigation.

### Changed

- Fresh projects and packaged builds now include required artifact schemas.
- Post-migration lifecycle and lookup paths now use TOML artifact layouts.
- RFC and clause files now use `[govctl]` metadata plus `[content]` sections.
- Migration planning is split into reusable migration steps instead of one monolithic plan.

### Removed

- Removed configurable `gov_root`; govctl now uses `gov/`.
- Removed inline `govctl.schema` fields from artifact metadata and JSON schema requirements.

### Fixed

- Fixed migrations so invalid converted artifacts fail before mixed-format partial writes are left behind.
- Fixed lifecycle-owned fields so generic `set` cannot mutate RFC, clause, ADR, or work-item status and release-owned fields.
- Fixed work-item completion so unresolved required guards block `work move done`.
- Fixed TUI scroll behavior for wrapped content and filtered lists.
- Fixed release/since metadata drift in migrated governance artifacts.

## [0.5.4] - 2026-03-14

### Added

- Added Claude plugin packaging, bundled skills/agents/hooks, and plugin marketplace metadata.
- Added `govctl check --has-active` for workflow gating.
- Added `wi` as an alias for the `work` subcommand.

## [0.5.3] - 2026-03-03

### Added

- Added version matching for both `X.Y.Z` and `vX.Y.Z` formats.
- Added a path-based semantic edit engine with strict parsing, typed edit plans, edit-operation SSOT metadata, and JSON/TOML adapters.
- Added generated or table-driven dispatch for resource get/set/add/remove path bindings.
- Added the migrate skill covering the ADR-0032 migration phases.

### Changed

- Reworked changelog rendering to preserve all known releases instead of regenerating only from the active release map.
- Migrated ADR, work, RFC, and clause edit execution to the V2 semantic edit engine.
- Updated ADR/work documentation with path-based edit examples.

### Removed

- Removed V1 manual edit dispatch from `edit.rs`.

### Fixed

- Fixed changelog inline-reference expansion, v-prefixed version matching, and preservation of releases not listed in `releases.toml`.
- Fixed work field help and invalid nested-path diagnostics.
- Fixed `remove` usage errors when indexed paths and matcher flags are mixed.

## [0.5.2] - 2026-02-24

### Fixed

- Fixed `work add journal`.
- Stabilized snapshot tests across days and machines.

## [0.5.1] - 2026-02-22

### Added

- Added work-item journals to schemas, models, and rendered output.
- Added ADR alternative details for pros, cons, and rejection reasons.
- Added bundled `discuss`, `gov`, `quick`, and `commit` skills.

### Changed

- Updated writer guidance, schema docs, and render output for journals and richer ADR alternatives.
- Updated resource get/set/add/remove command help with valid field lists.
- Unified skill template generation.

### Removed

- Removed the low-usage `commands/status.md` asset.

## [0.4.3] - 2026-02-16

### Added

- Added an exclusive gov-root lock for write commands, with bounded waits, clear timeout errors, and crash-safe release behavior.

### Changed

- Renamed `commands_dir` to `agent_dir` and changed the default from `.claude/commands` to `.claude`.

### Fixed

- Fixed user-facing path output so paths are displayed relative to the project root.

## [0.4.2] - 2026-02-13

### Fixed

- Made warning diagnostics include concise, actionable hints.

## [0.4.1] - 2026-02-11

### Added

- Added bundled writer skills for RFCs, ADRs, and work items.
- Added reviewer agents for RFCs, ADRs, work items, and compliance checks.

### Changed

- Moved command assets under `assets/commands/`.
- Renamed the CLI sync command while keeping `sync-commands` as a compatibility alias.

## [0.4.0] - 2026-02-07

### Added

- Added `rfc show`, `adr show`, `work show`, and `clause show`.
- Added structured JSON output for show commands and updated `describe` output accordingly.

### Changed

- Renamed `describe --format` to `describe --output`.

## [0.3.1] - 2026-02-07

### Added

- Added TUI headers, footers, breadcrumbs, counts, filters, quick-jump navigation, and detail scroll indicators.

### Changed

- Documented the TUI keymap and UX.

## [0.3.0] - 2026-01-29

### Added

- Added resource-specific render commands for RFCs, ADRs, and work items.

### Removed

- Removed the global render command's `--rfc-id` flag.

## [0.2.4] - 2026-01-29

### Fixed

- Made RFC, ADR, and work-item loading deterministic by sorting loaded artifacts by ID.

## [0.2.3] - 2026-01-26

### Added

- Added configurable ID strategies, including sequential, author-hash, and random IDs.
- Added coded diagnostics, `NO_COLOR` support, and confirmation prompts for deprecate/supersede commands.
- Added structured `--output`/`-o` formats and examples in command help.

### Changed

- Documented the new ID formats in schema documentation.

### Fixed

- Fixed TUI list scrolling so the selected row remains visible.
- Fixed RFC detail navigation into clauses and clause detail rendering.

## [0.2.2] - 2026-01-26

### Added

- Added the reference-hierarchy clause and `render changelog --force` regeneration mode.

### Changed

- Updated clause/RFC `since` handling and made default changelog rendering update only `Unreleased`.

### Fixed

- Fixed RFC inline-reference expansion and refs-field rendering.
- Fixed changelog rendering so missing release sections are inserted in order.

## [0.2.1] - 2026-01-25

### Added

- Added explicit changelog categories to work acceptance criteria.
- Added validation for acceptance criteria without a category.

### Fixed

- Fixed changelog inline-reference links and configurable docs output paths.

## [0.2.0] - 2026-01-19

### Added

- Added machine-readable `govctl describe` output and context-aware project-state suggestions.
- Added clause and work-item deletion commands with reference and lifecycle safety checks.
- Added shell completions for bash, zsh, fish, and PowerShell.
- Added artifact type auto-detection from IDs, optional type flags, and resource-first command coverage.
- Added RFC amendments, signatures, version bumping, and amended-RFC list indicators.
- Added list `--limit` shorthand and AI IDE asset synchronization.

### Changed

- Relaxed the frozen-normative validation constraint.
- Updated docs and command assets to use the resource-first CLI syntax.

### Removed

- Removed old verb-first commands.
- Removed `-n` from global `--dry-run` to avoid conflict with list limits.

### Fixed

- Fixed new clause creation so `since` is set from the parent RFC version.

## [0.1.0] - 2026-01-18

### Added

- Added the initial governance artifact model for RFCs, clauses, ADRs, work items, releases, changelog entries, signatures, references, and lifecycle state.
- Added core CLI flows for creating, editing, rendering, ticking, checking, releasing, and listing governance artifacts.
- Added structured work-item acceptance criteria, ADR alternatives, inline reference expansion, changelog categories, and Keep a Changelog rendering.
- Added a first TUI with dashboard, list, and detail views for core artifact types.
- Added source-reference scanning, placeholder-description warnings, coded diagnostics, colored output, and a snapshot-based test harness.
- Added initial bundled workflow guidance, README examples, and documentation fixes for the governed workflow.

### Changed

- Consolidated changelog categories into the shared model and updated schemas/templates to store them explicitly.
- Converted CLI failures from uncoded `anyhow` errors to structured diagnostics.

### Fixed

- Preserved backward compatibility for existing work items after checklist category migration.
- Fixed `mv` command help text.
