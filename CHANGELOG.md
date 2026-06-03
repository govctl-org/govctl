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
- Extract edit match option and index resolution helpers into a focused edit matcher module (WI-2026-06-01-001)
- Any remaining dead-code allowances in touched loop code are narrowly justified (WI-2026-06-01-002)
- Remaining edit dead-code allowances are narrowly justified (WI-2026-06-01-003)
- CLI check and describe behavior remains unchanged (WI-2026-06-01-004)
- Work dependency validation lives in a focused validate submodule (WI-2026-06-01-006)
- Existing validate public callers continue using crate::validate::{is_work_item_id, validate_work_dependencies} (WI-2026-06-01-006)
- Lifecycle transition helpers live in a focused validate submodule (WI-2026-06-01-007)
- Existing crate::validate lifecycle helper imports remain valid (WI-2026-06-01-007)
- Field edit validation lives in a focused validate submodule (WI-2026-06-01-008)
- Existing crate::validate field validation imports remain valid (WI-2026-06-01-008)
- Clause and work item deletion flow lives in a focused edit submodule (WI-2026-06-01-009)
- Existing cmd::edit delete entrypoints remain valid (WI-2026-06-01-009)
- Command router execution dispatch lives in a focused child module (WI-2026-06-01-010)
- Existing CommandPlan::execute behavior remains stable (WI-2026-06-01-010)
- Command router edit action construction lives in a focused child module (WI-2026-06-01-011)
- Existing resource_plan edit action helper imports remain valid (WI-2026-06-01-011)
- TOML target get/set/remove/tick helpers live in a focused cmd/edit child module (WI-2026-06-01-012)
- Work-item depends_on edits still run dependency loop validation (WI-2026-06-01-012)
- RFC and clause JSON edit target helpers live in a focused cmd/edit child module (WI-2026-06-01-013)
- Duplicated RFC and clause set-field implementation is consolidated without behavior changes (WI-2026-06-01-013)
- Shared edit document add/remove helpers live in a focused cmd/edit child module (WI-2026-06-01-014)
- TOML, JSON, and parent edit flows use the shared document target helpers (WI-2026-06-01-014)
- Nested edit runtime traversal, rendering, and mutation helpers live in a runtime child module (WI-2026-06-01-015)
- Existing callers continue using the same edit_runtime nested helper API (WI-2026-06-01-015)
- Loop execution-round orchestration and helper routines live in a loop command child module (WI-2026-06-01-016)
- Existing loop start/show/resume/run CLI behavior is preserved (WI-2026-06-01-016)
- Edit integration tests are split into artifact-focused included files (WI-2026-06-01-017)
- Existing edit integration test names and snapshot baselines remain stable (WI-2026-06-01-017)
- Command-router semantic plan types live in a child module (WI-2026-06-01-018)
- Existing command_router planning API and lock behavior are preserved (WI-2026-06-01-018)
- Shared CLI argument structs and value enums live in a cli child module (WI-2026-06-01-019)
- Existing CLI parse/help behavior is preserved (WI-2026-06-01-019)
- Artifact resource subcommand enums live in a cli child module (WI-2026-06-01-020)
- Top-level CLI command parse/help behavior is preserved (WI-2026-06-01-020)
- init-skills template and sync logic lives in a new.rs child module (WI-2026-06-01-021)
- cmd::new::sync_skills callers keep the same API (WI-2026-06-01-021)
- changelog rendering lives in a render child module (WI-2026-06-01-022)
- cmd::render::render_changelog callers keep the same API (WI-2026-06-01-022)
- cmd::lifecycle::cut_release callers keep the same API (WI-2026-06-01-023)
- release cutting logic lives in a lifecycle child module (WI-2026-06-01-023)
- ADR lifecycle public entrypoints keep the same API (WI-2026-06-01-024)
- ADR lifecycle logic lives in a lifecycle child module (WI-2026-06-01-024)
- RFC lifecycle logic lives in a lifecycle child module (WI-2026-06-01-025)
- RFC lifecycle public entrypoints keep the same API (WI-2026-06-01-025)
- edit set-field executor lives in a focused child module (WI-2026-06-01-026)
- lifecycle direct set and edit set public behavior are preserved (WI-2026-06-01-026)
- edit add-field executor lives in a focused child module (WI-2026-06-01-027)
- tag validation and work dependency loop checks remain on add paths (WI-2026-06-01-027)
- edit remove-field executor lives in a focused child module (WI-2026-06-01-028)
- indexed-path match flag validation remains shared with tick paths (WI-2026-06-01-028)
- edit tick-field executor lives in a focused child module (WI-2026-06-01-029)
- indexed-path match flag validation remains shared with remove paths (WI-2026-06-01-029)
- edit get-field dispatcher lives in a focused child module (WI-2026-06-01-030)
- public get behavior is preserved for all artifact types (WI-2026-06-01-030)
- ArtifactType lives in a focused edit artifact module (WI-2026-06-01-031)
- public cmd::edit::ArtifactType references remain compatible (WI-2026-06-01-031)
- nested render helpers live in a focused child module (WI-2026-06-01-032)
- nested get rendering remains unchanged for scalar, list, and object paths (WI-2026-06-01-032)
- nested traversal and indexing helpers live in a focused child module (WI-2026-06-01-033)
- nested add/set/remove/tick/get behavior remains unchanged (WI-2026-06-01-033)
- simple runtime rendering helpers live in a focused child module (WI-2026-06-01-034)
- simple get output remains unchanged for scalar, string-list, and status-list fields (WI-2026-06-01-034)
- simple runtime set and path mutation helpers live in a focused child module (WI-2026-06-01-035)
- simple set and list mutation behavior remains unchanged (WI-2026-06-01-035)
- simple runtime list and status-list operation bodies live in a focused child module (WI-2026-06-01-036)
- simple list add, set, remove, get, and tick behavior remains unchanged (WI-2026-06-01-036)
- nested runtime list mutation operation bodies live in a focused child module (WI-2026-06-01-037)
- nested add, remove, tick, and indexed set behavior remains unchanged (WI-2026-06-01-037)
- edit engine target resolution helpers live in a focused child module (WI-2026-06-01-038)
- request planning and target classification behavior remains unchanged (WI-2026-06-01-038)
- parsed CLI to command-plan routing lives in a focused child module (WI-2026-06-01-039)
- parsed command routing semantics remain unchanged (WI-2026-06-01-039)
- rendered RFC signature validation lives in a focused child module (WI-2026-06-01-040)
- project validation signature diagnostics remain unchanged (WI-2026-06-01-040)
- RFC and ADR bracket-reference hierarchy validation lives in a focused child module (WI-2026-06-01-041)
- bracket-reference hierarchy diagnostics remain unchanged (WI-2026-06-01-041)
- artifact refs validation lives in a focused child module (WI-2026-06-01-042)
- refs hierarchy and unknown-reference diagnostics remain unchanged (WI-2026-06-01-042)
- work item validation checks live in a focused child module (WI-2026-06-01-043)
- placeholder and legacy history diagnostics remain unchanged (WI-2026-06-01-043)
- artifact tag validation lives in a focused child module (WI-2026-06-01-044)
- tag format, allowed-tag, and guard-tag diagnostics remain unchanged (WI-2026-06-01-044)
- migration file operations live in a focused child module (WI-2026-06-01-045)
- migration preview, staging, commit, and rollback behavior remain unchanged (WI-2026-06-01-045)
- `TOML` rewrite planning lives in a focused migration child module (WI-2026-06-01-046)
- rewrite detection, schema-header insertion, and `govctl.schema` stripping remain unchanged (WI-2026-06-01-046)
- `RFC` `JSON`-to-`TOML` migration planning lives in a focused migration child module (WI-2026-06-01-047)
- mixed-storage, unexpected-file, missing-clause, and invalid-path diagnostics remain unchanged (WI-2026-06-01-047)
- release metadata migration planning lives in a focused migration child module (WI-2026-06-01-048)
- release `govctl.schema` normalization and validation diagnostics remain unchanged (WI-2026-06-01-048)
- RFC-0006 defines dynamic root-scope replan/add/remove semantics with dependency closure recomputation (WI-2026-06-01-081)
- RFC-0006 defines how existing loop item state is preserved or removed during scope mutation (WI-2026-06-01-081)
- RFC-0006 specifies canonical loop IDs as LOOP-YYYY-MM-DD-NNN and rejects plain-text loop IDs (WI-2026-06-01-081)
- Replanning recomputes dependency closure while preserving applicable item statuses and round counts (WI-2026-06-01-082)
- Skills instruct agents to create related work items first and execute them through one batch loop (WI-2026-06-01-083)
- Skills prefer generated LOOP IDs and avoid examples with hand-written plain-text loop IDs (WI-2026-06-01-083)
- Skills explain when to replan loop scope instead of starting scattered single-item loops (WI-2026-06-01-083)
- Reusable loop integration test setup, command, schema, and work item helpers live outside tests/test_loop.rs (WI-2026-06-01-084)
- Loop replan/add/remove handling lives in one cohesive helper module (WI-2026-06-01-085)
- Loop start/show/resume/run behavior, command routing, diagnostics, and help snapshots remain unchanged (WI-2026-06-01-085)
- Unnecessary dead-code suppressions or stale cleanup comments are removed or narrowed (WI-2026-06-01-086)
- Retained compatibility comments name the current behavior they protect (WI-2026-06-01-086)
- repeated TOML read, validate, and deserialize logic is shared by ADR, guard, work item, and release loaders (WI-2026-06-01-088)
- repeated schema-header write and dry-run preview logic is shared by ADR, guard, work item, and release writers (WI-2026-06-01-088)
- `src/loop_planner.rs` is replaced by a `src/loop_planner/` module root without same-name coexistence (WI-2026-06-01-091)
- loop planner tests live outside the production module body (WI-2026-06-01-091)
- dependency closure and cycle detection helpers are isolated from loop state preservation logic (WI-2026-06-01-092)
- deterministic loop execution ordering is provided through a reusable graph helper (WI-2026-06-01-092)
- LoopCommand is defined in a focused src/cli child module and re-exported through the existing cli facade (WI-2026-06-01-093)
- Root CLI module no longer carries loop subcommand variant bodies (WI-2026-06-01-093)
- Loop command module root contains command entrypoints without list rendering or state discovery helper bodies (WI-2026-06-01-094)
- Loop state discovery and generated ID helpers are reusable by start, resume, and run without importing through unrelated command entrypoints (WI-2026-06-01-094)
- edit add dispatch uses a request struct instead of a long positional parameter list (WI-2026-06-01-095)
- edit field dispatch uses a request struct instead of a long positional parameter list (WI-2026-06-01-096)
- src/cmd/edit/toml_target.rs is replaced by src/cmd/edit/toml_target/mod.rs without same-name file and directory coexistence (WI-2026-06-01-097)
- work dependency edit detection and validation live in a focused TOML target child module and remain used by add and set flows (WI-2026-06-01-097)
- TOML target tick path matching and status-list mutation logic lives in a focused child module (WI-2026-06-01-098)
- TOML target module root keeps tick load/write orchestration without detailed matcher branches (WI-2026-06-01-098)
- loop root help presents loop list as the recovery/discovery step before resume (WI-2026-06-01-100)
- repo-local gov and wi-writer skills mention loop list for discovering current loops (WI-2026-06-01-100)
- src/cmd/edit/path.rs is replaced by src/cmd/edit/path/mod.rs without same-name file and directory coexistence (WI-2026-06-01-101)
- edit path parser tests live in src/cmd/edit/path/tests.rs outside the production module body (WI-2026-06-01-101)
- edit engine planning tests live in src/cmd/edit/engine/tests.rs (WI-2026-06-01-102)
- edit engine module root keeps production planning logic without inline test bulk (WI-2026-06-01-102)
- src/cmd/loop_cmd/execution.rs is replaced by src/cmd/loop_cmd/execution/mod.rs without same-name file and directory coexistence (WI-2026-06-01-103)
- loop run state selection helpers live in src/cmd/loop_cmd/execution/run_state.rs (WI-2026-06-01-103)
- per-work-item loop round execution helpers live in src/cmd/loop_cmd/execution/round.rs (WI-2026-06-01-104)
- loop execution module root keeps high-level run orchestration without round-helper bulk (WI-2026-06-01-104)
- src/command_router/execute.rs is replaced by src/command_router/execute/mod.rs without same-name file and directory coexistence (WI-2026-06-01-105)
- command execution scope extraction helpers live in src/command_router/execute/scope.rs (WI-2026-06-01-105)
- built-in command execution dispatch lives in src/command_router/execute/builtin.rs (WI-2026-06-01-106)
- command router execution root keeps semantic plan operation handlers without built-in dispatch bulk (WI-2026-06-01-106)
- list resource handlers live in src/cmd/list/resources.rs (WI-2026-06-01-107)
- src/cmd/list/mod.rs remains a small dispatcher without per-resource filtering bulk (WI-2026-06-01-107)
- list summary row models live in src/cmd/list/summaries.rs (WI-2026-06-01-108)
- list resource handlers use summary row helpers without duplicating row conversion closures (WI-2026-06-01-108)
- RFC and clause field validation behavior remains unchanged (WI-2026-06-01-109)
- DiagnosticCode and DiagnosticLevel live in a focused diagnostic code module (WI-2026-06-01-110)
- existing crate::diagnostic imports continue to compile without caller changes (WI-2026-06-01-110)
- TUI list filter cache logic lives in a focused app filter helper module (WI-2026-06-01-111)
- list filter behavior and selection bounds remain unchanged (WI-2026-06-01-111)
- TUI navigation, scroll, and clause-selection helpers live in a focused app navigation module (WI-2026-06-01-112)
- existing event and renderer call sites continue to use App without behavior changes (WI-2026-06-01-112)
- CLI output formatting is split into focused ui submodules without changing crate::ui call sites (WI-2026-06-01-114)
- color capability detection and diagnostic formatting remain reusable through crate::ui (WI-2026-06-01-114)
- generated EDIT_RULES_VERSION is test-only without dead_code suppression (WI-2026-06-01-115)
- new work item creation initializes content without explicitly constructing an empty legacy inline history field (WI-2026-06-01-116)
- new-command artifact constructors live in focused child modules with a small dispatch root (WI-2026-06-01-117)
- show_rfc, show_adr, show_work, and show_clause live in a focused render::show module with public reexports unchanged (WI-2026-06-01-118)
- CLI common action/status, target/output, and reusable argument definitions live in focused child modules (WI-2026-06-01-119)
- src/config.rs is moved to src/config/mod.rs with no same-name file/directory coexistence (WI-2026-06-01-120)
- IdStrategy behavior lives in a focused config child module and remains re-exported through crate::config (WI-2026-06-01-121)
- Config load/path helpers and default TOML rendering are separated from config data type definitions (WI-2026-06-01-121)
- src/load.rs is moved to src/load/mod.rs with no same-name file/directory coexistence (WI-2026-06-01-122)
- RFC and clause file loading logic lives in a focused load child module (WI-2026-06-01-123)
- project-index loading and warning aggregation live in a focused load child module (WI-2026-06-01-123)
- changelog rendering lives in src/cmd/render/changelog/mod.rs with no same-name changelog.rs sibling (WI-2026-06-01-124)
- public cmd::render::render_changelog callers keep the same API (WI-2026-06-01-124)
- changelog item grouping and release-section rendering live in focused helper modules (WI-2026-06-01-125)
- existing release parsing, version matching, and newest-first ordering are isolated from render orchestration (WI-2026-06-01-125)
- verification runner lives in src/verification/runner/mod.rs with no same-name runner.rs sibling (WI-2026-06-01-126)
- crate::verification::run_guard remains the public runner entrypoint (WI-2026-06-01-126)
- guard output capture lives in a focused runner helper module (WI-2026-06-01-127)
- guard process-group setup and termination helpers live in a focused runner helper module (WI-2026-06-01-127)
- edit JSON-target code lives in src/cmd/edit/json_target/mod.rs with no same-name json_target.rs sibling (WI-2026-06-01-128)
- current json_target caller imports remain compatible (WI-2026-06-01-128)
- JSON-target get and full-document display logic lives in a focused helper module (WI-2026-06-01-129)
- JSON-target set operations share a focused helper module with unchanged validation and touch behavior (WI-2026-06-01-129)
- JSON-target simple-list add/remove operations live in a focused helper module (WI-2026-06-01-129)
- RFC status phase and clause reference validation helpers live in a focused validate::rfc module (WI-2026-06-01-130)
- validate_project keeps current validation order and diagnostics (WI-2026-06-01-130)
- release metadata validation lives in a focused validate::releases module (WI-2026-06-01-131)
- crate::validate::validate_releases remains available to callers (WI-2026-06-01-131)
- TOML-target get and resolved-target rendering live in a focused helper module (WI-2026-06-01-132)
- TOML-target remove helpers live in a focused helper module with unchanged removal notifications (WI-2026-06-01-132)
- TOML-target set and apply helpers live in a focused helper module (WI-2026-06-01-133)
- work item depends_on set validation remains unchanged (WI-2026-06-01-133)
- UI messages live under src/ui/messages/mod.rs with no same-name messages.rs sibling (WI-2026-06-01-134)
- existing crate::ui message function imports remain compatible (WI-2026-06-01-134)
- common file render check and dry-run output helpers live in a focused messages helper module (WI-2026-06-01-135)
- artifact field lifecycle changelog and release message helpers live in a focused messages helper module (WI-2026-06-01-135)
- shared validate reference hierarchy helper owns RFC ADR Work Item authority ordering (WI-2026-06-01-136)
- structured refs validation keeps existing diagnostic codes and messages (WI-2026-06-01-136)
- bracket refs use the shared reference hierarchy helper for RFC and ADR authority violations (WI-2026-06-01-137)
- bracket refs keep link-specific diagnostic wording and scan coverage (WI-2026-06-01-137)
- TUI header footer breadcrumb and keybinding rendering live in a focused chrome child module (WI-2026-06-01-138)
- main TUI draw keeps view dispatch and public rendering behavior unchanged (WI-2026-06-01-138)
- TUI help overlay and centered popup geometry live in a focused help child module (WI-2026-06-01-139)
- view-specific help content and overlay rendering stay unchanged (WI-2026-06-01-139)
- tag command is normalized to a directory module without same-name file and directory coexistence (WI-2026-06-01-140)
- cmd::tag public functions and tag regex access remain compatible for current callers (WI-2026-06-01-140)
- tag registry TOML helpers live in a focused child module (WI-2026-06-01-141)
- tag usage counting and tag list output formatting are separated from command orchestration (WI-2026-06-01-141)
- loop state validation is normalized to a directory module without same-name file and directory coexistence (WI-2026-06-01-142)
- validation public and storage-facing helper access remains compatible (WI-2026-06-01-142)
- canonical loop ID and lifecycle transition validation live in focused child modules (WI-2026-06-01-143)
- loop state and round record contract validation are separated from shared diagnostic helpers (WI-2026-06-01-143)
- TagCommand lives in the cli resources module and is re-exported through the existing resource CLI namespace (WI-2026-06-01-144)
- tag command parsing, routing, and help text remain unchanged (WI-2026-06-01-144)
- SkillFormat lives in the cli common module and remains available through the existing cli re-export (WI-2026-06-01-145)
- init-skills command parsing and skill installation callers remain unchanged (WI-2026-06-01-145)
- per-work-item loop round execution and its private status/retry helpers live in a focused execution item module (WI-2026-06-01-146)
- loop round orchestration keeps dependency readiness and finalization behavior unchanged (WI-2026-06-01-146)
- verify and loop item execution share one exact work-item ID lookup helper (WI-2026-06-01-147)
- render/edit path matching and not-found scopes remain outside this helper (WI-2026-06-01-147)
- loop state and round-record writes share one private TOML persistence helper (WI-2026-06-01-148)
- loop state file paths, round file paths, dry-run output, and diagnostics remain unchanged (WI-2026-06-01-148)
- command router tests are grouped into focused child modules (WI-2026-06-01-149)
- no src/command_router/tests.rs and src/command_router/tests/ coexistence remains (WI-2026-06-01-149)
- simple and nested edit-runtime list removals share one matched-index removal helper (WI-2026-06-01-150)
- removal order, returned text order, matcher errors, and edit behavior remain unchanged (WI-2026-06-01-150)
- render tests are split into domain-focused modules without a same-name file and directory coexisting (WI-2026-06-01-151)
- link, ADR, work item, and legacy inline history rendering assertions remain covered (WI-2026-06-01-151)
- edit action tests live in the existing command_router test module instead of the production edit_action module (WI-2026-06-01-152)
- action count, stdin inference, selector rejection, tick, remove, and explicit empty-string coverage remains unchanged (WI-2026-06-01-152)
- changelog category rendered labels and rendered-prefix matching are owned by ChangelogCategory (WI-2026-06-01-153)
- edit matching keeps stripping only existing rendered category prefixes for non-regex patterns (WI-2026-06-01-153)
- src/write/changelog resolves as a module directory with parser tests in a sibling test module (WI-2026-06-01-154)
- existing changelog parser behavior and write module re-exports remain covered (WI-2026-06-01-154)
- cli/mod.rs keeps the Cli root while top-level Commands lives in cli/commands.rs (WI-2026-06-01-155)
- command names, aliases, help text, and command_router Commands imports remain unchanged (WI-2026-06-01-155)
- model/mod.rs keeps declarations and exports while model tests live in model/tests.rs (WI-2026-06-01-156)
- existing model constructor, default, AsRef, category helper, and entry accessor tests remain covered (WI-2026-06-01-156)
- nested list mutation shares one private helper for root resolution, descent, item-rule lookup, and array access (WI-2026-06-01-162)
- nested add, remove, tick, and set behavior and diagnostics remain unchanged (WI-2026-06-01-162)
- force and dry-run preview still bypass confirmation without changing command behavior (WI-2026-06-01-163)
- delete, deprecate, and supersede commands use one shared destructive confirmation helper (WI-2026-06-01-163)
- clause adapter missing-field diagnostics and scope paths remain unchanged (WI-2026-06-01-164)
- ClauseJsonAdapter and ClauseTomlAdapter share one clause document loading helper (WI-2026-06-01-164)
- ADR, work item, and guard TOML adapters share one load-and-find helper (WI-2026-06-01-165)
- adapter-specific match predicates and not-found diagnostics remain unchanged (WI-2026-06-01-165)
- direct command-router invariant failures return Diagnostic errors instead of uncoded anyhow messages (WI-2026-06-01-166)
- file I/O and serialization helpers used by CLI commands map failures to Diagnostic values (WI-2026-06-01-166)
- remaining intentional anyhow boundaries are documented as transport or test-only, not user-facing errors (WI-2026-06-01-166)
- Diagnostic exposes a reusable helper for E0901 filesystem failures with action and scope context (WI-2026-06-01-167)
- CLI-visible filesystem call sites reuse the helper without changing diagnostic code or scope semantics (WI-2026-06-01-167)
- edit command internal invariant failures use E0903 unexpected-error diagnostics instead of E0901 I/O diagnostics (WI-2026-06-01-168)
- repeated edit target-shape diagnostic construction is centralized without adding a new module (WI-2026-06-01-168)
- scan command and core execution paths for non-Diagnostic error propagation and classify remaining intentional boundaries (WI-2026-06-01-169)
- convert in-scope anyhow/String/std error returns to Diagnostic or Diagnostics without changing CLI behavior (WI-2026-06-01-169)
- command planning and router entry functions use DiagnosticResult where their error values are Diagnostics (WI-2026-06-01-170)
- main error handling no longer depends on downcasting anyhow errors for converted router paths (WI-2026-06-01-170)
- edit engine planning APIs return DiagnosticResult instead of anyhow::Result (WI-2026-06-01-171)
- command_router resolves edit scope without Diagnostic::from_anyhow (WI-2026-06-01-171)
- cmd/edit root no longer defines shared edit action and request helper types directly (WI-2026-06-01-172)
- edit value input resolution is extracted and reused by add, clause edit, and field edit paths without behavior changes (WI-2026-06-01-172)
- global render and artifact render execution reuse shared render helper functions (WI-2026-06-01-173)
- command-router execute module avoids new file splits while reducing repeated render legacy_command calls (WI-2026-06-01-173)
- production command or rendering boundary returns DiagnosticResult instead of anyhow where errors are diagnostics (WI-2026-06-01-175)
- CLI error reporting no longer needs to downcast anyhow for the cleaned boundary (WI-2026-06-01-175)
- edit command modules use DiagnosticResult and Diagnostic instead of anyhow for diagnostic errors (WI-2026-06-01-176)
- command router calls migrated edit paths without legacy anyhow downcast (WI-2026-06-01-176)
- lifecycle and creation commands return DiagnosticResult for diagnostic errors (WI-2026-06-01-177)
- command router calls migrated lifecycle and creation paths without legacy anyhow downcast (WI-2026-06-01-177)
- external command paths map IO, network, archive, terminal, and parser failures to Diagnostic (WI-2026-06-01-178)
- production source no longer needs Diagnostic::from_anyhow for command execution (WI-2026-06-01-178)
- RFC-0002 classifies loop as a project-level local execution-state command namespace (WI-2026-06-01-179)
- RFC-0006 defines loop command-surface semantics without positional argument overloading (WI-2026-06-01-179)
- lifecycle dispatcher delegates clause RFC and ADR supersede/deprecate logic to artifact modules (WI-2026-06-01-184)
- RFC and clause artifact read/write paths share private helper functions without changing public APIs (WI-2026-06-01-185)
- flat RFC and clause normalization reuses common key-moving helpers (WI-2026-06-01-185)
- RFC and clause document edit adapters live outside the common adapter core (WI-2026-06-01-186)
- Existing cmd::edit::adapter imports continue to compile through re-exports (WI-2026-06-01-186)
- ADR, work item, and guard TOML edit adapters live outside the common adapter core (WI-2026-06-01-187)
- Shared adapter core owns only common adapter contracts and document container types (WI-2026-06-01-187)
- ClauseSpec, ClauseWire, ClauseKind, and ClauseStatus live in src/model/clause.rs (WI-2026-06-01-188)
- Existing crate::model clause type imports continue to compile through model re-exports (WI-2026-06-01-188)
- ChangelogEntry and ChangelogCategory live in src/model/changelog.rs (WI-2026-06-01-189)
- Work item and RFC model files import changelog types from the shared model module (WI-2026-06-01-189)
- Generic artifact read/write pipeline lives outside src/write/artifact.rs (WI-2026-06-01-190)
- Public RFC/clause read and write functions keep their existing crate::write exports (WI-2026-06-01-190)
- Legacy RFC/clause normalizers live outside src/write/artifact.rs (WI-2026-06-01-191)
- Loader-facing normalizer exports remain available from crate::write (WI-2026-06-01-191)
- RFC and clause lifecycle commands share path resolution helpers (WI-2026-06-01-192)
- Existing legacy JSON migration-required diagnostics are preserved (WI-2026-06-01-192)
- Pending clause version update logic lives outside rfc lifecycle action handlers (WI-2026-06-01-193)
- RFC bump and finalize still update clauses with missing since values (WI-2026-06-01-193)
- Loop start/list/resume/schema tests live outside tests/test_loop.rs (WI-2026-06-01-194)
- tests/test_loop.rs remains a small integration-test entrypoint (WI-2026-06-01-194)
- Loop run and target tests live outside tests/test_loop.rs (WI-2026-06-01-195)
- Loop scope add/remove/replan tests live outside tests/test_loop.rs (WI-2026-06-01-195)
- RFC lifecycle tests live outside tests/test_lifecycle.rs without snapshot renames (WI-2026-06-01-197)
- tests/test_lifecycle.rs remains a small lifecycle integration-test entrypoint (WI-2026-06-01-197)
- Clause lifecycle tests live outside tests/test_lifecycle.rs without snapshot renames (WI-2026-06-01-198)
- ADR lifecycle tests live outside tests/test_lifecycle.rs without snapshot renames (WI-2026-06-01-198)
- tests/test_errors.rs remains a small error-test entrypoint (WI-2026-06-01-199)
- Invalid artifact schema error tests live outside tests/test_errors.rs (WI-2026-06-01-199)
- Work legacy-history and dependency error tests live outside tests/test_errors.rs (WI-2026-06-01-200)
- RFC and clause wire-format error tests live outside tests/test_errors.rs without snapshot renames (WI-2026-06-01-200)
- ADR, Work, and Clause detail views share one markdown panel renderer (WI-2026-06-01-201)
- Detail view scroll, wrapping, titles, and border colors remain unchanged (WI-2026-06-01-201)
- Loop state storage uses `loop.work` for the editable work set and a separate non-editable resolved dependency closure field (WI-2026-06-01-202)
- Loop add/remove help and behavior prefer `work`, accept `wi`, and reject `work_items` as a field name (WI-2026-06-01-202)
- Status command uses shared private helpers for status counting and breakdown printing (WI-2026-06-01-203)
- Status output, color behavior, phase display, totals, and active-work section remain unchanged (WI-2026-06-01-203)
- RFC, ADR, and work item signature computation share private helper code for common hasher/update/finalize steps (WI-2026-06-01-204)
- Signature output and amended-RFC behavior remain unchanged (WI-2026-06-01-204)
- Nested list remove and tick paths share private helper code for converting list entries to matchable text (WI-2026-06-01-205)
- Nested list add/remove/tick/set behavior and diagnostics remain unchanged (WI-2026-06-01-205)
- TOML-backed add operations share private helper code for serialize/mutate/deserialize document updates (WI-2026-06-01-206)
- Tag add validation is isolated from the main add dispatch flow without changing tag error behavior (WI-2026-06-01-206)
- Simple runtime list get/remove/tick paths share private helpers for immutable array lookup and item text extraction (WI-2026-06-01-207)
- Simple list add/remove/tick/set/get behavior and diagnostics remain unchanged (WI-2026-06-01-207)
- Clause and work item deletion guards share private project reference collection helper code (WI-2026-06-01-208)
- Delete command safeguards, confirmation prompts, and diagnostics remain unchanged (WI-2026-06-01-208)
- Source scan include and exclude glob construction share private helper code (WI-2026-06-01-209)
- Source scan matching and invalid pattern diagnostics remain unchanged (WI-2026-06-01-209)
- RFC, ADR, and work item TUI list views share private helpers for table rendering, status cells, and tag cells (WI-2026-06-01-210)
- TUI list table headers, constraints, colors, status icons, and row contents remain unchanged (WI-2026-06-01-210)
- Resource list commands share private tag-filter helper code (WI-2026-06-01-211)
- List filtering, tag matching, sorting, limits, and output rows remain unchanged (WI-2026-06-01-211)
- RFC-0006 requires the loop `wi` field alias for `work` scope mutation (WI-2026-06-01-212)
- Loop tests continue to accept `wi` and reject `work_items`/`root_work_items` (WI-2026-06-01-212)
- Source scanning and structured reference validation share artifact ID index construction (WI-2026-06-02-001)
- Reference existence and outdated-source-scan diagnostics remain unchanged (WI-2026-06-02-001)
- RFC, ADR, and work item refs validation use a shared private helper (WI-2026-06-02-002)
- Existing reference hierarchy and unknown-ref diagnostic codes and messages remain unchanged (WI-2026-06-02-002)
- Indexed remove and tick paths share exact-index match option construction (WI-2026-06-02-003)
- Indexed remove and tick behavior and diagnostics remain unchanged (WI-2026-06-02-003)
- Tick target handling shares private simple and nested dispatch helpers (WI-2026-06-02-004)
- Tick root and indexed-path matching semantics remain unchanged (WI-2026-06-02-004)
- JSON and TOML edit get paths share ResolvedTarget rendering helper code (WI-2026-06-02-005)
- JSON get still rejects nested targets and TOML get still renders nested targets (WI-2026-06-02-005)
- RFC, ADR, work item, and clause show commands share output-mode printing logic (WI-2026-06-02-006)
- show command artifact lookup, JSON diagnostics, and markdown rendering behavior stay unchanged (WI-2026-06-02-006)
- RFC, ADR, and work item render commands share collection selection and summary control flow (WI-2026-06-02-007)
- render empty-set messages, not-found diagnostics, dry-run behavior, and write calls stay unchanged (WI-2026-06-02-007)
- status command header, total, phase, and active-work rendering share a local color-aware helper (WI-2026-06-02-008)
- status command plain output and snapshot-visible formatting remain unchanged (WI-2026-06-02-008)
- edit runtime simple-list get and render paths share scalar item formatting (WI-2026-06-02-009)
- edit runtime status-list get and render paths share status line formatting (WI-2026-06-02-009)
- clause and work-item delete safeguards share referrer collection and sorting (WI-2026-06-02-010)
- delete safeguard error codes, messages, and deletion behavior remain unchanged (WI-2026-06-02-010)
- RFC lifecycle commands share display-path-aware RFC write helper (WI-2026-06-02-011)
- RFC version bump signature refresh keeps existing best-effort behavior (WI-2026-06-02-011)
- ADR and work item signature functions share the simple-artifact hashing flow (WI-2026-06-02-012)
- RFC signature computation remains clause-aware and behavior-preserving (WI-2026-06-02-012)
- RFC and clause loaders share source-file read error handling (WI-2026-06-02-013)
- clause lookup helpers share clause ID splitting and source path construction (WI-2026-06-02-014)
- edit target removal shares simple and nested removal helper flows (WI-2026-06-02-015)
- Nested edit runtime shares repeated path traversal and structured-list diagnostic helpers without adding new modules (WI-2026-06-02-016)
- list resource commands share one local summary-output helper without adding modules (WI-2026-06-02-017)
- TUI dashboard status cards share one local counting helper without adding modules (WI-2026-06-02-018)
- clause create, show, delete, and lifecycle paths share clause ID splitting instead of hand-parsing (WI-2026-06-02-019)
- clause and work-item delete tests live in focused include files (WI-2026-06-02-020)
- work edit tests are grouped into focused include files (WI-2026-06-02-021)
- path edit tests are grouped into focused include files (WI-2026-06-02-022)
- loop execution tests are grouped into focused include files (WI-2026-06-02-023)
- changelog workflow tests are grouped into broad include files (WI-2026-06-02-024)
- loop surface tests are grouped into focused concern files (WI-2026-06-02-025)
- RFC lifecycle tests are grouped into command-area files (WI-2026-06-02-026)
- RFC/clause error tests are grouped into focused concern files (WI-2026-06-02-027)
- lock integration tests are grouped into basic and concurrency files (WI-2026-06-02-028)
- ADR gate tests are grouped by decision and accept gate concerns (WI-2026-06-02-029)
- ADR gate tests share normalization and gate assertion helpers (WI-2026-06-02-029)
- deletion referrer scanning is extracted into a focused reusable helper module without changing delete command behavior (WI-2026-06-02-030)
- src/signature.rs is replaced by src/signature/mod.rs without same-name file/directory coexistence (WI-2026-06-02-031)
- signature module root no longer mixes signature orchestration with canonical JSON serialization (WI-2026-06-02-031)
- src/cmd/lifecycle/rfc.rs remains focused on bump/finalize/advance and shared RFC lifecycle helpers (WI-2026-06-02-032)
- RFC supersede validation and write logic live in a focused flat lifecycle module (WI-2026-06-02-032)
- target remove matching and notification helpers live in a focused flat edit helper module (WI-2026-06-02-033)
- target_doc.rs remains focused on target rendering and add behavior without same-name directory coexistence (WI-2026-06-02-033)
- loop command routing and execution internals use work_id/work_ids naming instead of work_items field-style names (WI-2026-06-02-034)
- loop work scope mutation still documents and accepts work plus wi while rejecting work_items/root_work_items (WI-2026-06-02-034)
- render-specific command-router execution helpers move out of execute/mod.rs into a focused sibling module (WI-2026-06-02-035)
- command-router render behavior and unsupported render diagnostics stay unchanged (WI-2026-06-02-035)
- artifact tag add and project tag validation share tag registry helpers for format and allowed-set checks (WI-2026-06-02-036)
- tag add/check diagnostics and messages remain stable (WI-2026-06-02-036)
- guard delete and show reuse a shared guard lookup helper for not-found diagnostics (WI-2026-06-02-037)
- guard delete reference checks are isolated from command output and write flow (WI-2026-06-02-037)
- command plan lock disposition delegates read-only classification to a focused helper (WI-2026-06-02-038)
- new artifact creation uses one shared TOML write helper for RFC, ADR, work item, and clause files (WI-2026-06-02-039)
- new artifact diagnostics, schema headers, dry-run writes, and creation output remain stable (WI-2026-06-02-039)
- list-like JSON array output uses one shared pretty-print fallback helper (WI-2026-06-02-040)
- command JSON output paths use one shared diagnostic pretty-printer helper (WI-2026-06-02-041)
- JSON serialization diagnostics and stdout output remain stable (WI-2026-06-02-041)
- command TOML output paths use one shared diagnostic pretty-printer helper (WI-2026-06-02-042)
- TOML serialization diagnostics and stdout output remain stable (WI-2026-06-02-042)
- self-update tests live in a sibling test module without same-name directory shape (WI-2026-06-02-043)
- self-update production module keeps the same public/private behavior (WI-2026-06-02-043)
- migrate ops rollback tests live in a sibling test module without same-name directory shape (WI-2026-06-02-044)
- migration file-operation rollback behavior remains covered (WI-2026-06-02-044)
- TUI inline tests live in sibling test modules without changing production module names (WI-2026-06-02-045)
- TUI renderer and project-load test coverage remains equivalent after extraction (WI-2026-06-02-045)
- command utility inline tests live in sibling test modules without changing production APIs (WI-2026-06-02-046)
- CLI aliases and edit field semantics remain unchanged (WI-2026-06-02-046)
- edit mutation planning target extraction is shared across set/add/remove/tick call sites (WI-2026-06-02-047)
- edit diagnostics and field ownership semantics remain unchanged (WI-2026-06-02-047)
- render show commands use shared lookup/not-found helper where behavior is identical (WI-2026-06-02-048)
- render show markdown/JSON output and relative-path diagnostics remain unchanged (WI-2026-06-02-048)
- ADR lifecycle lookup and not-found scaffolding is shared across completeness, accept, reject, and supersede paths (WI-2026-06-02-049)
- ADR lifecycle diagnostics and transition/write behavior remain unchanged (WI-2026-06-02-049)
- display path tests are split into focused command-area case files following existing include-based test organization (WI-2026-06-02-050)
- display path test snapshots, helper assertions, and covered command cases remain unchanged (WI-2026-06-02-050)
- RFC and clause supersede paths share lifecycle replacement path lookup helpers (WI-2026-06-02-051)
- lifecycle migration-required errors and replacement not-found diagnostics remain unchanged (WI-2026-06-02-051)
- clause deletion uses a local helper to unlink clause references from RFC sections (WI-2026-06-02-052)
- clause deletion diagnostics, safeguards, and dry-run output remain unchanged (WI-2026-06-02-052)
- edit runtime field specification types live in a focused spec module (WI-2026-06-02-053)
- generated edit-runtime field table continues to compile through the same runtime lookup behavior (WI-2026-06-02-053)
- shared edit runtime support helpers live in a focused support module (WI-2026-06-02-054)
- simple and nested edit runtime diagnostics and rendering remain unchanged (WI-2026-06-02-054)
- Codex agent template generation lives in a focused build-support module (WI-2026-06-02-055)
- generated Codex agent template output remains unchanged (WI-2026-06-02-055)
- edit-ops SSOT model and validation helpers live in focused build-support code (WI-2026-06-02-056)
- generated edit rules/runtime output and SSOT validation behavior remain unchanged (WI-2026-06-02-056)
- integration tests use normal Rust modules instead of `include!` for split test suites (WI-2026-06-02-057)
- existing integration-test coverage and insta snapshot names remain stable after module conversion (WI-2026-06-02-057)
- no test file under `tests/` contains `include!(` after the cleanup (WI-2026-06-02-058)
- integration snapshot helper logic is shared instead of repeated across test roots (WI-2026-06-02-059)
- existing integration snapshot file names and contents remain stable (WI-2026-06-02-059)
- Shared guard fixture helpers replace duplicated guard-writing code in guard and verify integration tests (WI-2026-06-02-060)
- Shared helper internals remove duplicate command execution and transcript formatting (WI-2026-06-02-061)
- Verification timeout helper uses shared command transcript formatting (WI-2026-06-02-062)
- Lock integration tests share timeout-config and queue-work-item fixture setup (WI-2026-06-02-063)
- loop guard helper reuses the shared canonical guard writer with timeout support (WI-2026-06-02-064)
- guard integration tests are organized into non-include sibling modules by command concern (WI-2026-06-02-065)
- tag integration tests are organized into non-include sibling modules by existing concern (WI-2026-06-02-066)
- source-scan tests share project and source-file fixture helpers without splitting the suite (WI-2026-06-02-067)
- move tests share project/date and active-work fixture helpers without splitting the suite (WI-2026-06-02-068)
- common integration helper returns initialized temp project with normalized current date (WI-2026-06-02-069)
- representative integration suites use the shared project-date fixture without file splitting (WI-2026-06-02-069)
- common integration helpers expose deterministic work item ID formatting (WI-2026-06-02-070)
- touched integration suites use shared work item ID helpers instead of local first-ID formatting (WI-2026-06-02-070)
- tag integration tests use the shared project-date fixture helper (WI-2026-06-02-071)
- display path tests use shared project-date and work-id fixture helpers (WI-2026-06-02-072)
- work delete tests use shared project-date and work-id fixture helpers (WI-2026-06-02-073)
- clause delete tests use shared project-date and work-id fixture helpers (WI-2026-06-02-074)
- RFC lifecycle cases use the shared project-date fixture helper (WI-2026-06-02-075)
- ADR and clause lifecycle tests use the shared project-date fixture helper (WI-2026-06-02-076)
- RFC edit tests use the shared project-date fixture helper (WI-2026-06-02-077)
- ADR edit tests use the shared project-date fixture helper (WI-2026-06-02-078)
- Clause and path-case edit tests use the shared project-date fixture helper (WI-2026-06-02-079)
- Work edit tests use the shared project-date fixture helper (WI-2026-06-02-080)
- Error tests use the shared project-date fixture helper (WI-2026-06-02-081)
- Source-scan and RFC lifecycle tests use the shared project-date fixture helper (WI-2026-06-02-082)
- Plain temp-date fixture helper is available for integration tests (WI-2026-06-02-083)
- No-project describe test uses the shared temp-date helper (WI-2026-06-02-083)
- Help tests use one local snapshot runner helper (WI-2026-06-02-084)
- Loop surface and scope tests use the shared project-date fixture helper (WI-2026-06-02-085)
- Loop execution tests use the shared project-date fixture helper (WI-2026-06-02-086)
- ADR gate test helper uses the shared project-date fixture helper (WI-2026-06-02-087)
- Loop schema test helpers share schema validation setup through one private helper (WI-2026-06-02-088)
- Loop command item/dependency lookups share private helper functions (WI-2026-06-02-089)
- Render and show commands share artifact not-found diagnostic construction (WI-2026-06-02-090)
- RFC and clause lifecycle path helpers share TOML lookup fallback logic (WI-2026-06-02-091)
- Edit document adapters share private load helper logic (WI-2026-06-02-092)
- Tag new and delete membership checks avoid temporary String allocation (WI-2026-06-02-093)
- Tag commands and validation share one allowed-tag membership helper (WI-2026-06-02-094)
- Describe context suggestions share local construction helper (WI-2026-06-02-095)
- Simple and nested edit rendering use a shared scalar-list text helper (WI-2026-06-02-100)
- ID strategy author-hash and random suffix generation share one hex formatter (WI-2026-06-02-101)
- Default owner and author-hash ID generation share one git config lookup helper (WI-2026-06-02-102)
- Signature public function comments use accurate diagnostic wording (WI-2026-06-02-103)
- Signature hex encoding avoids per-byte format allocation (WI-2026-06-02-103)
- loop command work ID validation uses one shared command helper for loop work fields and run targets (WI-2026-06-02-104)
- loop scope add and remove reuse shared root-set construction without changing output or diagnostics (WI-2026-06-02-104)
- TUI ADR and work list views render through a shared resource-list-row component with explicit id/title/status/tags inputs (WI-2026-06-02-105)
- TUI RFC detail metadata uses shared header-line components for label/value, status, phase, and optional collection fields (WI-2026-06-02-105)
- TUI list/detail/dashboard shared render pieces use explicit component-style structs instead of anonymous helper flow (WI-2026-06-02-106)
- TUI refactor keeps files cohesive and does not introduce include-based test splitting or same-name file/directory module pairs (WI-2026-06-02-106)
- status command summary sections share a reusable section helper instead of repeating header/status/total flow (WI-2026-06-02-107)
- status cleanup stays within the existing module and preserves current status snapshot output (WI-2026-06-02-107)
- RFC and clause loaders share a local source parse-normalize-validate helper (WI-2026-06-02-108)
- loader refactor preserves current wire-format error snapshots and legacy JSON/TOML behavior (WI-2026-06-02-108)
- TUI detail views share a component-level viewport result for scroll clamping and footer status (WI-2026-06-02-109)
- TUI refactor preserves existing dashboard, list, and detail rendering behavior without adding new TUI module splits (WI-2026-06-02-109)
- edit add handler accepts an already-resolved value string and no longer resolves stdin/value itself (WI-2026-06-02-110)
- edit add request types remain internal and preserve current add diagnostics and write behavior (WI-2026-06-02-110)
- command-router parsed planning delegates loop command conversion to a local helper (WI-2026-06-02-111)
- command-router parsed planning delegates tag command conversion without adding new module splits (WI-2026-06-02-111)
- TUI ADR, work item, and clause detail views render through a shared markdown detail panel component adapter (WI-2026-06-02-112)
- TUI detail refactor preserves current detail output, scroll footer behavior, and module boundaries without new TUI splits (WI-2026-06-02-112)
- source-scan integration tests share local helpers for repeated RFC setup and check snapshot execution (WI-2026-06-02-113)
- source-scan production comments no longer point to obsolete test fixture locations (WI-2026-06-02-113)
- move-command integration tests share a local normalized snapshot assertion while preserving existing snapshot names (WI-2026-06-02-114)
- move-command tests remove comments that only restate test names (WI-2026-06-02-114)
- RFC detail metadata header rendering uses a reusable TUI component instead of inline widget construction (WI-2026-06-02-115)
- RFC detail clause list rendering uses a reusable TUI component while preserving clause selection behavior (WI-2026-06-02-115)
- tests/common exposes shared current-test and normalized-command snapshot assertion helpers (WI-2026-06-02-116)
- describe and happy-path integration tests use the shared normalized snapshot helper without changing snapshot names (WI-2026-06-02-116)
- integration-test helper macros delegate current-test snapshot assertions to the shared helper (WI-2026-06-02-117)
- snapshot names and integration test module layout remain unchanged (WI-2026-06-02-117)
- test_source_scan and test_rfc_lifecycle use the shared current-test snapshot helper (WI-2026-06-02-118)
- existing snapshot names and integration test module layout remain unchanged (WI-2026-06-02-118)
- init and agent-dir tests remove leftover debug eprintln output (WI-2026-06-02-119)
- comments that only restate init and agent-dir test names or obvious setup steps are removed (WI-2026-06-02-119)
- ResourceTable exposes a reusable filtered-index row assembly path for TUI resource lists (WI-2026-06-02-120)
- RFC, ADR, and work item list renderers keep resource-specific row content while dropping duplicate collect-and-render scaffolding (WI-2026-06-02-120)
- TUI test support exposes a reusable render harness for TestBackend drawing and buffer capture (WI-2026-06-02-121)
- TUI renderer tests use the shared harness while keeping test-specific setup and assertions local (WI-2026-06-02-121)
- list table output computes stdout color support once per render instead of through a redundant local wrapper (WI-2026-06-02-122)
- list table, plain, and JSON output behavior is preserved (WI-2026-06-02-122)
- TUI rendering behavior and existing component call sites are preserved (WI-2026-06-02-123)
- TUI component definitions are grouped by coherent component family modules rather than one mixed catalog file (WI-2026-06-02-123)
- dashboard summary counts, labels, and layout are preserved (WI-2026-06-02-124)
- dashboard summary code delegates reusable card and row rendering to the summary component family (WI-2026-06-02-124)
- snapshot naming, normalization, and command execution helpers live in focused common test helper modules (WI-2026-06-02-125)
- existing integration test helper API and snapshot macros remain compatible (WI-2026-06-02-125)
- temp-project, date/id, guard, and guarded-work fixtures live in a focused common fixture helper module (WI-2026-06-02-126)
- existing integration tests continue to use the stable common helper surface (WI-2026-06-02-126)
- operational top-level command after_help bodies live in a focused CLI help module (WI-2026-06-02-127)
- CLI command definitions and help output remain behaviorally unchanged (WI-2026-06-02-127)
- command enum shape and top-level help behavior are preserved (WI-2026-06-02-128)
- resource and workflow top-level command after_help bodies live in the CLI help module (WI-2026-06-02-128)
- TUI header and footer chrome rendering use explicit render component structs with narrow inputs (WI-2026-06-02-129)
- ui::draw remains the app shell composition point and rendered TUI behavior is preserved (WI-2026-06-02-129)
- DiagnosticCode enum and diagnostic metadata mapping are separated into coherent modules (WI-2026-06-02-130)
- DiagnosticCode::code and DiagnosticCode::level preserve existing public behavior (WI-2026-06-02-130)
- CLI status and TUI dashboard share a reusable status-count helper instead of local duplicate counters (WI-2026-06-02-131)
- CLI status output and TUI dashboard summary counts preserve existing behavior (WI-2026-06-02-131)
- Extract shared TUI header/footer bar framing into a component-style helper (WI-2026-06-02-132)
- Share edit runtime list text and object mutation helpers across simple and nested list handling (WI-2026-06-02-133)
- Share repeated RFC/clause wire-format test fixture setup without splitting the suite into tiny files (WI-2026-06-02-134)
- Header and footer behavior and keybinding text remain unchanged (WI-2026-06-02-135)
- Generic chrome bar rendering lives in the TUI component layer (WI-2026-06-02-135)
- Clause edit tests share repeated RFC and clause creation commands (WI-2026-06-02-136)
- Existing clause edit scenarios and snapshot call sites remain intact (WI-2026-06-02-136)
- Clause delete tests share local RFC and clause fixture writers (WI-2026-06-02-137)
- Existing clause delete scenarios and snapshot call sites remain intact (WI-2026-06-02-137)
- Existing loop targeting scenarios and assertions remain intact (WI-2026-06-02-138)
- Loop targeting tests share local command builders (WI-2026-06-02-138)
- Existing loop execution scenarios and assertions remain intact (WI-2026-06-02-139)
- Loop execution tests use shared command builders from common loop helpers (WI-2026-06-02-139)
- Loop scope and listing tests use shared command builders for repeated loop operations (WI-2026-06-02-140)
- Existing loop scope and listing scenarios and assertions remain intact (WI-2026-06-02-140)
- Loop start, validation, and guard tests use shared command builders for repeated loop operations (WI-2026-06-02-141)
- Existing loop start, validation, and guard scenarios and assertions remain intact (WI-2026-06-02-141)
- ADR nested path tests use local command helpers for repeated ADR setup and nested path commands (WI-2026-06-02-142)
- Existing ADR nested path scenarios and snapshots remain intact (WI-2026-06-02-142)
- TUI list and metadata status rendering share one explicit status text component (WI-2026-06-02-143)
- TUI component structure remains cohesive without include-based test splitting or same-name file/module pairs (WI-2026-06-02-143)
- Work acceptance tests use local command helpers for repeated work ID and acceptance_criteria operations (WI-2026-06-02-144)
- Existing work acceptance scenarios and snapshots remain intact (WI-2026-06-02-144)
- Work reference tests use local command helpers for repeated refs and depends_on operations (WI-2026-06-02-145)
- Existing work reference and dependency scenarios remain intact (WI-2026-06-02-145)
- Work field tests use local command helpers for repeated work ID and field operations (WI-2026-06-02-146)
- Existing work field scenarios and snapshots remain intact (WI-2026-06-02-146)
- Detail-only TUI panel and metadata helpers live with the detail view renderer instead of the shared components module (WI-2026-06-02-147)
- Existing TUI detail rendering, footer scroll status, and snapshots remain unchanged (WI-2026-06-02-147)
- Describe command catalog uses current noun-first command names and examples for artifact commands (WI-2026-06-02-148)
- Describe command catalog includes loop list/start/show/resume/add/remove/run metadata aligned with RFC-0006 (WI-2026-06-02-148)
- Describe snapshots update only for catalog metadata while project-state suggestions remain semantically unchanged (WI-2026-06-02-148)
- Generic dynamic command and work-item test builders live in tests/common/commands.rs instead of loop_helpers (WI-2026-06-02-149)
- Changelog integration tests reuse shared work command builders for work new/add/tick flows (WI-2026-06-02-149)
- Loop tests keep loop-specific helper semantics while importing generic work builders from common commands (WI-2026-06-02-149)
- Common test command helpers expose reusable work get/set/add/remove/show/list/tick builders used by edit work tests (WI-2026-06-02-150)
- Edit work field, reference, and acceptance tests remove duplicate local command/work-id builders while retaining scenario-specific helpers (WI-2026-06-02-150)
- Nested ADR edit-path tests reuse the common dynamic command builder instead of defining a local duplicate (WI-2026-06-02-151)
- ADR-specific nested path scenario helpers remain local and no new abstraction or file split is introduced (WI-2026-06-02-151)
- Move, delete, and happy-path integration tests reuse common dynamic command builders instead of manual Vec<String> command assembly (WI-2026-06-02-152)
- Scenario-specific test setup helpers remain local and no new test files or include-based split is introduced (WI-2026-06-02-152)
- loop_state storage functions are re-exported without trivial forwarding wrappers (WI-2026-06-02-155)
- Nested edit status-list detection is shared through edit rules (WI-2026-06-02-156)
- RFC, ADR, and work markdown writers share the inline-reference expansion/write helper (WI-2026-06-02-157)
- Loop scope mutation tests use explicit work and wi command helpers instead of a generic supported-field helper (WI-2026-06-02-158)
- Display-path edit tests share one local RFC fixture helper for set and bump dry-run cases (WI-2026-06-02-160)
- RFC clause error check tests share local RFC and clause JSON fixture writers (WI-2026-06-02-161)
- command list table setup is shared through a reusable output helper instead of repeated table initialization (WI-2026-06-02-163)
- Config exposes shared RFC directory/source and clause directory/source path helpers (WI-2026-06-02-164)
- production load, lifecycle, new, edit, and render paths reuse the shared helpers where they use canonical RFC/clause layout (WI-2026-06-02-164)
- loop run and loop work-scope mutation paths use a shared terminal-state validation helper (WI-2026-06-03-001)
- loop resume uses the shared terminal-state validation helper (WI-2026-06-03-002)
- migration write and delete apply branches share one local backup helper (WI-2026-06-03-003)
- state-only loop helper functions are private to the state module (WI-2026-06-03-004)
- narration-only comments are removed from selected command handlers (WI-2026-06-03-005)
- shared integration snapshot settings no longer suppress expression metadata globally (WI-2026-06-03-006)
- low-level test command-builder helpers are private when only higher-level helpers use them (WI-2026-06-03-007)
- describe catalog prerequisite literals are shared through named constants (WI-2026-06-03-008)
- TOML rewrite planning uses a shared local helper for directory, RFC, and release write operations (WI-2026-06-03-009)
- simple and nested edit-runtime mutation paths use one shared JSON path creation helper (WI-2026-06-03-010)
- changelog preservation test reuses local helpers for repeated absolute-path, legacy-version, and version-heading assertions (WI-2026-06-03-011)
- RFC, ADR, and work item TUI list filters use one local query-matching helper (WI-2026-06-03-012)
- Small CLI/tag-only helper modules are merged where they add no reusable boundary (WI-2026-06-03-013)
- Edit target handlers no longer use thin get/remove micro modules (WI-2026-06-03-013)
- Loop execution and validation micro helpers are consolidated without behavior changes (WI-2026-06-03-013)
- RFC-0006 defines loop run as a local execution protocol that records and validates round evidence instead of implementing code or directly completing work items (WI-2026-06-03-014)
- loop state and round schemas model loop-level execution rounds, summaries, verification evidence, and note candidates while preserving existing work, depends_on, guards, and notes ownership (WI-2026-06-03-014)
- loop command behavior reuses start/list/show/resume/run/add/remove/replan without adding a parallel resource CRUD model or new testing system (WI-2026-06-03-014)
- embedded gov and wi-writer skills plus CLI describe/help guidance avoid journal and direct agents to loop state for execution trace (WI-2026-06-03-014)

### Removed

- Journal cannot be fetched, added, edited, ticked, or removed as a path-addressable work item field (WI-2026-05-31-003)
- Delete duplicate test-only match and removal helper code from the edit command file (WI-2026-06-01-001)
- Stale crate-level dead_code suppressions are removed from loop modules (WI-2026-06-01-002)
- Edit helper dead-code suppressions that are no longer needed are removed (WI-2026-06-01-003)
- Internal command plan no longer stores unused check or describe fields (WI-2026-06-01-004)
- Unused UI formatter helpers are deleted (WI-2026-06-01-005)
- Broad ui module dead_code allowance is removed (WI-2026-06-01-005)
- `src/theme.rs` no longer has a module-wide `dead_code` allowance (WI-2026-06-01-080)
- clippy too_many_arguments suppression is removed from src/cmd/edit/add.rs (WI-2026-06-01-095)
- clippy too_many_arguments suppression is removed from src/cmd/edit/mod.rs (WI-2026-06-01-096)
- inert internal scope_override argument plumbing is removed while the hidden CLI compatibility flag remains accepted (WI-2026-06-01-096)
- unused field-validation ArtifactKind variants are deleted (WI-2026-06-01-109)
- unused optional_string set-mode schema, generator, and runtime branches are removed (WI-2026-06-01-113)
- unused diagnostic variants E0205, E0903, W0104, W0105, W0111, and W0112 are removed from the enum and code maps (WI-2026-06-01-161)
- DiagnosticCode no longer needs a dead_code allowance (WI-2026-06-01-161)
- Legacy loop state keys `root_work_items` and `work_items` are not accepted for the unreleased loop schema (WI-2026-06-01-202)
- Misleading TUI components/detail.rs module split is removed (WI-2026-06-02-147)

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
- loop command docs consistently name the editable field work and document wi as its alias (WI-2026-06-01-183)

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
