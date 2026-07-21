# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Release entries are curated summaries for readers. Work item traceability remains in
`gov/releases.toml`.

## [Unreleased]

## [0.14.1] - 2026-07-21

0.14.1 removes unnecessary commit churn from the bundled agent workflows.

### Fixed

- Implementation commits may remain traceable to either active or completed
  work items; agents no longer create or reactivate work items solely to make a
  commit.
- Work-item notes, acceptance criteria, and closure are finalized before the
  final implementation commit so the implementation and its governance record
  can be committed together.
- Quick changes keep their work item active until every acceptance criterion is
  satisfied.

## [0.14.0] - 2026-07-21

0.14.0 separates concise current-state views from complete governance history.
Human-readable `show` output no longer presents deprecated or superseded bodies
as current requirements, while explicit history and structured formats preserve
the full record.

### Added

- Added `show --history` for RFCs, ADRs, Clauses, Work Items, and Guards to
  restore complete human-readable history on demand.
- Added complete YAML and TOML structured output alongside JSON for every
  supported `show` resource.

### Changed

- Human-readable `show` now defaults to a current-state projection. Deprecated
  RFCs and superseded ADRs become metadata-only views, while deprecated or
  superseded Clause text is omitted without hiding lifecycle and replacement
  metadata.
- RFC rendering now includes authoritative owners, tags, and supersession
  metadata in one compact metadata block.

### Compatibility

- `render` remains a complete archival projection, and structured `show`
  remains equivalent to complete `get` retrieval. Combining `--history` with a
  structured output format is rejected because structured output is already
  complete.

## [0.13.0] - 2026-07-20

0.13.0 makes each RFC version a single, explicit authoring candidate. It rejects
attempts to bump away from an unsealed `spec` candidate, seals exactly the
content that enters `impl`, and adds a safe way to remove mistaken Clauses before
that boundary.

### Added

- `govctl clause delete` now permits deletion of an unreferenced Clause
  introduced in the current normative `spec` candidate when its `since` version
  matches the RFC version. Clauses inherited from earlier versions still require
  deprecation or supersession.

### Changed

- Version-changing `govctl rfc bump` operations now open the next candidate only
  from a normative RFC in `impl`, `test`, or `stable` with a sealed signature and
  a content amendment. A second version-changing bump from `spec` is rejected.
- Advancing from `spec` to `impl` establishes the current RFC and Clause content
  as the sealed implementation baseline. Candidate edits made during `spec` are
  no longer reported as unversioned amendments.

### Fixed

- Deprecated and superseded Clause output now shows lifecycle status before
  historical normative text, including the direct replacement for superseded
  Clauses.
- Version bumps and later phase transitions now reject missing sealed signatures
  without modifying RFC or pending-Clause state, with guidance to migrate or
  restore the baseline.
- Later phase transitions reject legacy projection signatures with migration
  guidance instead of silently replacing the sealed baseline.
- Clause deletion now detects structured references, inline governance links,
  and supersession edges, reports every referencing artifact, and applies the
  RFC index update and Clause removal transactionally.

### Upgrade Notes

- Workflows that previously issued another version-changing bump while an RFC
  was already in `spec` must now continue editing that candidate and advance it
  to `impl`. Open the following version only after a later content amendment.
- Repositories with an RFC in `impl`, `test`, or `stable` but no trustworthy
  sealed signature must run migration or restore a consistent RFC-and-Clause
  baseline before further lifecycle progression.

## [0.12.0] - 2026-07-16

0.12.0 completes the current-version RFC authoring model. Normative RFCs can be
refined throughout `spec` and sealed on entry to `impl` without a bookkeeping
bump, while current changelog corrections remain version-preserving operations.

### Added

- Added `govctl rfc get <id> changelog` to expose the unique changelog entry
  matching the RFC's current version as a single object.
- Added controlled current-changelog editing for summaries and categorized
  changes without exposing historical entries or lifecycle-owned fields.

### Changed

- RFC and Clause content can now be refined throughout `spec`; advancing to
  `impl` seals the final current-version content as the implementation baseline.
- Version-changing `govctl rfc bump` operations now require normative status.
  Draft RFCs use finalization as their publication boundary, and deprecated RFCs
  remain terminal.
- `govctl rfc bump --change` without a bump level is explicitly changelog-only
  and preserves the RFC version, phase, changelog date, and amendment signature.
- Clause `since` assignment now follows the RFC lifecycle: draft Clauses are
  assigned at finalization, normative `spec` Clauses at creation, and later
  Clauses at the next content-changing bump.

### Fixed

- RFC and Clause edits made during `spec` no longer require an empty follow-up
  bump before the RFC can advance to `impl`.
- Current-changelog operations resolve entries by version and reject missing,
  duplicate, historical, or lifecycle-owned targets without mutation.
- Entering `impl` now rejects unresolved pending Clause versions instead of
  silently rewriting their provenance.
- Failed lifecycle mutations restore governed-file path existence and byte
  content across create, delete, and overwrite operations.

## [0.11.0] - 2026-07-16

0.11.0 makes lifecycle corrections explicit and failure-safe. RFC amendments
restart their implementation lifecycle, unreleased work can be reopened, and an
accidental latest local release cut can be undone without rewriting history.

### Added

- Added `govctl release undo <expected-version>` to retract only the newest
  local release cut, with guarded version matching and rollback-safe writes.
- ADR validation now rejects renderer-owned headings in proposed ADR source and
  reports the conflicting field and heading, including under `--force`.

### Changed

- Content-changing `govctl rfc bump` operations now start the new RFC version at
  `spec`; phase-only, changelog-only, and signature-baseline updates preserve
  the current phase.
- Unreleased `done` Work Items can return to `active`, while Work Items already
  referenced by a release remain immutable.
- `govctl rfc finalize` now exposes only the `normative` target; deprecation
  remains a separate lifecycle operation.

### Fixed

- Failed lifecycle mutations now preserve the original RFC, Clause, Work Item,
  and release data instead of leaving partial multi-file updates.
- Legacy RFC signature migration now establishes content-only amendment
  baselines without changing versions, phases, or changelog history.
- Release validation now rejects invalid dates, missing Work Item references,
  and Work Item IDs assigned to more than one release.
- RFC and Clause transition errors now identify valid next states or explain
  when the artifact is terminal.
- ADR projection checks now ignore headings inside fenced code blocks, compare
  formatted titles by visible text, and preserve historical terminal ADRs.
- ADR placeholder context now uses its own `W0113` warning instead of the
  missing-reference warning code.

### Known Limitations

- In 0.11.0, changing RFC or Clause content after a version bump has entered
  `spec` requires another bump before advancing to `impl`. Later releases seal
  the final current-version content during the `spec` to `impl` transition.

## [0.10.2] - 2026-07-05

0.10.2 restores compatibility between the bundled Agent Skills and GitHub
Copilot CLI 1.0.65+ by correcting `argument-hint` metadata types.

### Fixed

- Quoted bracket-style `argument-hint` values so YAML parsers read them as
  strings and load the affected skills correctly.

## [0.10.1] - 2026-06-29

0.10.1 is a lifecycle correctness and write-safety patch. Clause supersession
now preserves direct history across replacement chains, validates replacement
targets consistently, and avoids partial updates when lifecycle commands fail.

### Fixed

- Clause supersession now preserves direct replacement history, so chains such
  as `A -> B -> C` remain valid after B is superseded or deprecated.
- `govctl clause supersede` now accepts active same-RFC shorthand and qualified
  cross-RFC replacements while rejecting missing, self-referential, and cyclic
  targets.
- Supersession validation now handles very large acyclic graphs without
  exhausting the call stack.
- RFC bump and finalize operations now stop before making changes when a
  pending clause cannot be read or parsed.
- Lifecycle writes are now failure-atomic: failed multi-file operations restore
  earlier changes, and diagnostics report any incomplete rollback.

## [0.10.0] - 2026-06-17

0.10.0 tightens loop execution semantics and governance validation. Loop rounds
now behave strictly as audit checkpoints rather than retry budgets, RFC bumps
reject empty amendments, and bare-reference warnings point reviewers to the
source prose that needs attention.

### Changed

- `govctl loop run` no longer treats `round_count` as a failure signal for
  loops, work items, or dependency readiness.

### Removed

- Removed `--max-rounds` from `govctl loop run` and stopped persisting
  `max_rounds` in new round artifacts. Repeated execution limits now belong to
  callers rather than govctl loop state.

### Fixed

- `govctl rfc bump` now rejects empty version bumps when no RFC or clause
  content changed since the stored amendment signature.
- Changelog-only updates no longer make a later RFC version bump valid, while
  actual RFC or clause content amendments still permit the next bump.
- `W0112` bare-reference diagnostics now include the scanned artifact field,
  field-local line, and short source context so reviewers can locate raw
  artifact IDs quickly.

## [0.9.5] - 2026-06-12

0.9.5 is a reviewer-evidence and compatibility patch. It gives reviewer agents
source-level diagnostics for raw artifact IDs, fills out the work-item
verification edit surface, and fixes project-root path handling when `check` is
run from a subdirectory.

### Added

- `govctl check` now warns with `W0112` when reviewable governed prose
  mentions a known artifact ID without `[[...]]` inline reference syntax, so
  reviewers can rely on source diagnostics instead of rendered markdown.

### Fixed

- Reviewer agent guidance now treats raw reference syntax as source-sensitive
  and defers to `govctl check` diagnostics rather than inferring syntax from
  rendered output.
- Work-item verification fields are now available through the canonical
  get/edit surface, including `verification.required_guards` add/remove support
  and `verification.waivers` guard/reason updates.
- `govctl check` now resolves project-support files and source scanning from
  the project root even when invoked from a subdirectory, avoiding false
  `W0111` warnings and missed source references.

## [0.9.4] - 2026-06-09

0.9.4 is a workflow hygiene and distribution patch. It tightens agent guidance
around artifact authority boundaries, installs complete skill bundles for local
agents, fixes cargo-binstall release metadata, and makes stale loop plans
visible before agents keep working from outdated dependency state.

### Added

- `govctl loop show` and `govctl loop list` now report whether a persisted loop
  plan is fresh or stale against current Work Item dependencies.

### Changed

- Distributed writer skills and reviewer agents now make RFC, ADR, and Work Item
  authority boundaries more explicit, so agents put requirements, decisions,
  and execution tracking in the right artifacts.
- `init-skills` now installs full skill directory bundles, including bundled
  references and assets, while still excluding plugin/global-only onboarding
  skills.

### Fixed

- `govctl loop run` now rejects stale stored dependency closures before opening
  another round and points users to `govctl loop replan`.
- cargo-binstall metadata now matches the actual GitHub Release assets for both
  Unix `.tar.gz` archives and Windows `.zip` archives.

## [0.9.3] - 2026-06-07

0.9.3 ships the first TUI v2 read-only cockpit. The TUI now gives humans a
denser project overview, artifact discovery, loop-state inspection with DAG
context, search, and diagnostics while keeping all mutation owned by the CLI.

### Added

- Added the TUI v2 cockpit, with dashboard navigation across overview,
  artifact browsing, search, loops, and diagnostics.
- Added loop views that list persisted loop states and render selected loop
  dependency DAGs with item status, selected-work context, and readable
  terminal-size fallbacks.
- Added TUI search and diagnostics views so users can find governed artifacts
  and triage `govctl check` output from the cockpit without mutating project
  state.
- Added RFC and ADR coverage for the TUI v2 architecture, including the
  read-only boundary, responsibility separation, and the decision to defer full
  CRUD editing.

### Fixed

- `govctl check` now reports schema and load diagnostics directly instead of
  printing a successful-looking checked-count summary first.
- TUI cockpit state now stays consistent after sorted supplement loading, empty
  search submissions, loop-state read failures, DAG fallback/error handling,
  and ops summary rendering.
- TUI no longer enables mouse capture when it does not handle mouse events,
  preserving normal terminal selection and copy behavior.

## [0.9.2] - 2026-06-05

0.9.2 focuses on discovery and lookup performance. It adds a project-wide
search command backed by disposable local state, and it speeds up common
single-artifact commands without making local indexes authoritative.

### Added

- Added `govctl search` for ranked discovery across RFCs, clauses, ADRs, work
  items, and verification guards.
- Search can filter by artifact type and tag, limit results, rebuild the local
  index with `--reindex`, and emit table, JSON, or plain output.
- Search indexes are stored under `.govctl/` and refreshed before results are
  returned, so stale local cache data is never treated as source of truth.

### Changed

- Single-artifact RFC, ADR, work item, guard, and clause lookups now use direct
  path resolution or a local artifact catalog where possible, avoiding broad
  scans on common commands.
- Catalog entries are validated against the target artifact ID before use,
  repaired when stale, and bypassed when they cannot be trusted.

## [0.9.1] - 2026-06-04

0.9.1 is a patch release for upgrade safety after 0.9.0. It tightens
reference validation and makes stale local project support state visible before
users hit confusing schema or local-state failures.

### Fixed

- `govctl check` now catches lower-authority artifact references written as bare
  IDs in governed RFC and ADR text, closing the loophole where removing
  `[[...]]` delimiters avoided hierarchy validation.
- `govctl check` now warns when bundled JSON Schema files under `gov/schema/`
  are missing or stale, even if `gov/config.toml` already reports the current
  schema version.
- `govctl migrate` now refreshes missing `.govctl.lock` and `.govctl/`
  `.gitignore` entries for existing projects, matching the local-state
  protection that fresh `govctl init` projects already receive.
- `govctl check` now reports missing govctl-managed local-state `.gitignore`
  entries and points users to `govctl migrate`.

## [0.9.0] - 2026-06-04

### Added

- Added the first local loop workflow. `govctl loop start`, `show`, `resume`, `run`, `list`, `add`, `remove`, and `replan` now operate on persisted loop state under `.govctl/loops/` instead of writing execution trace into work item fields.
- Added canonical loop IDs (`LOOP-YYYY-MM-DD-NNN`), deterministic loop listing, lifecycle filters, and table/plain/json output for local loop state discovery.
- Added work item dependencies through `depends_on`, including display support in `work show`/render output and validation for malformed, unknown, or cyclic dependencies.
- Added dependency-aware loop planning. Loops compute an execution order from `depends_on`, preserve existing item state where possible, and mark downstream items blocked when dependencies fail or cancel.
- Added loop state and round schemas for execution evidence, summaries, verification evidence, and note candidates.
- Added non-blocking diagnostics for work items that still contain legacy inline execution-history entries, with guidance to move durable takeaways to notes and keep new execution trace in loop state.

### Changed

- Work item execution history is no longer modeled as an editable field. New work items omit `content.journal`; existing legacy entries still render correctly in `work show` and generated work item output.
- Loop scope mutation now uses `work` as the editable field, keeps `wi` as the supported alias, and rejects legacy `work_items`/`root_work_items` field names.
- `loop run` is specified as a local execution protocol that records and validates round evidence. It does not introduce a parallel resource CRUD model or a separate testing system.
- CLI-visible command routing, file I/O, serialization, and scan paths now return coded `Diagnostic` values where possible. Remaining `anyhow` boundaries are documented as transport or test-only.
- Embedded skills, agent guidance, CLI help, and `describe` output now direct execution trace to loop state and durable takeaways to notes.
- Integration tests were converted from `include!`-based splitting to normal Rust modules, and several over-split helper modules were folded back into their callers.

### Removed

- Removed path-addressable work item journal operations. `journal` can no longer be fetched, added, edited, ticked, or removed as a work item field.
- Removed support for legacy loop state keys `root_work_items` and `work_items`.
- Removed legacy RFC/clause JSON storage compatibility from normal operation. Repositories that still contain `rfc.json` or clause JSON files now fail with `E0505` and must be migrated with govctl `<0.9` before upgrading.
- Removed RFC/clause JSON conversion from `govctl migrate`. The command now upgrades TOML artifacts and schema metadata only.

### Fixed

- `govctl migrate` now treats the config version bump as part of the same transactional operation set as artifact rewrites, and rollback restores overwritten or deleted files if apply fails.
- `govctl rfc supersede` now updates the source RFC, records the replacement, and rejects missing or invalid replacements with diagnostics.
- `govctl clause delete` now refuses to delete clauses referenced by another artifact while preserving the existing draft-status and section-update behavior for safe deletions.
- `refs` edits now validate target existence and RFC/ADR reference hierarchy before writing, including indexed `refs[N]` updates.
- Acceptance-criteria ticking now matches category-prefixed patterns.
- Commit skill behavior now detects whether a repository is governed before running `govctl` commands, so non-governed repositories skip governance checks while governed repositories keep the full workflow.

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
