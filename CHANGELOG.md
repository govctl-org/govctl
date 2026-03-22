# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- govctl check enforces refs + [[...]] hierarchy (WI-2026-03-22-001)

## [0.7.0] - 2026-03-17

### Changed

- init no longer dumps skills/agents (WI-2026-03-17-010)
- init prints hint about init-skills and plugin (WI-2026-03-17-010)
- sync renamed to init-skills in CLI (WI-2026-03-17-010)

### Fixed

- migrate overwrites schema JSON files with bundled versions (WI-2026-03-17-010)

## [0.6.0] - 2026-03-17

### Added

- Runtime validation uses JSON Schemas for RFC, clause, ADR, work item, and release artifacts (WI-2026-03-16-001)
- govctl check validates releases as first-class artifacts (WI-2026-03-16-001)
- govctl migrate converts legacy JSON RFC and clause files to TOML and rewrites govctl-managed clause paths (WI-2026-03-16-002)
- govctl migrate upgrades legacy releases.toml files to include schema metadata (WI-2026-03-16-002)
- Guard artifacts define reusable executable verification checks (WI-2026-03-17-004)
- gov/config.toml can enable guard enforcement and define default guard requirements (WI-2026-03-17-004)
- Work items can require extra guards and record structured waivers instead of raw disable flags (WI-2026-03-17-004)
- All generated TOML artifacts include #:schema comment headers with deterministic relative paths (WI-2026-03-17-005)
- Migration handles legacy JSON and old-format TOML to header-based spec-aligned TOML (WI-2026-03-17-005)
- Generic stage/backup/commit/rollback engine operates on Vec<FileOp> (WI-2026-03-17-006)
- config.toml [schema] version bumped to 2 after successful migration (WI-2026-03-17-006)
- Migration step v2->v3 strips govctl.schema from existing files (WI-2026-03-17-007)
- src/theme.rs with SemanticColor enum and backend adapters (WI-2026-03-17-008)
- Deprecated/superseded uses Muted (DarkGrey) consistently across CLI and TUI (WI-2026-03-17-008)
- show commands render styled markdown via markdown-to-ansi (WI-2026-03-17-008)
- show output strips HTML comments, anchors, and converts relative links (WI-2026-03-17-008)
- Ctrl+d/u half-page and PgUp/PgDn full-page scroll in TUI (WI-2026-03-17-008)
- TUI feature default-enabled in Cargo.toml (WI-2026-03-17-008)
- ansi-to-tui dependency behind tui feature flag (WI-2026-03-17-009)
- render_to_tui_text helper in terminal_md.rs (WI-2026-03-17-009)

### Changed

- Fresh projects and packaged builds include required artifact schema files (WI-2026-03-16-001)
- post-migration lifecycle and lookup paths no longer assume .json RFC/clause files (WI-2026-03-16-002)
- RFC/clause TOML uses [govctl] metadata + [content] sections (spec alignment) (WI-2026-03-17-005)
- MigrationStep struct + FileOp enum replace monolithic MigrationPlan (WI-2026-03-17-006)
- Existing v0.5->v0.6 migration extracted into plan_v1_to_v2 (WI-2026-03-17-006)
- draw_adr_detail uses markdown pipeline (WI-2026-03-17-009)
- draw_work_detail uses markdown pipeline (WI-2026-03-17-009)
- draw_clause_detail uses markdown pipeline (WI-2026-03-17-009)

### Removed

- Configurable gov_root from config; hardcode to gov/ (WI-2026-03-17-005)
- govctl.schema field from AdrMeta, WorkItemMeta, GuardMeta, ReleasesMeta, RfcMeta, ClauseMeta (WI-2026-03-17-007)
- schema from required in all 6 JSON schemas (WI-2026-03-17-007)

### Fixed

- migrated repositories fail if any converted artifact would be invalid instead of leaving mixed-format partial writes (WI-2026-03-16-002)
- C-RELEASE-DEF has an explicit since version (WI-2026-03-17-001)
- WI-2026-03-01-001 no longer uses placeholder description text (WI-2026-03-17-001)
- clause since is no longer directly settable through govctl clause set (WI-2026-03-17-002)
- C-RELEASE-DEF receives its since version via RFC-0000 bump rather than direct mutation (WI-2026-03-17-002)
- generic set rejects RFC lifecycle-owned fields like version, status, and phase (WI-2026-03-17-003)
- generic set rejects clause lifecycle-owned or edit-owned fields like text, status, superseded_by, and since (WI-2026-03-17-003)
- generic set rejects ADR/work status fields and tick-owned nested status fields (WI-2026-03-17-003)
- work move done rejects unresolved required guards (WI-2026-03-17-004)
- TUI scroll position accounts for word-wrap (WI-2026-03-17-008)
- list_indices() cached per frame, invalidated on filter change (WI-2026-03-17-008)

## [0.5.4] - 2026-03-14

### Added

- Plugin manifest (.claude-plugin/marketplace.json) with correct metadata (WI-2026-03-04-001)
- Plugin directory with skills, agents, and hooks (WI-2026-03-04-001)
- SessionStart hook injects govctl status context (WI-2026-03-04-001)
- Stop hook warns if govctl check has pending failures (WI-2026-03-04-001)
- govctl check --has-active exits 0 when active work item exists (WI-2026-03-04-003)
- govctl check --has-active exits non-zero when no active work item (WI-2026-03-04-003)
- CLI accepts 'wi' as alias for work subcommand (WI-2026-03-09-001)

## [0.5.3] - 2026-03-03

### Added

- Support both X.Y.Z and vX.Y.Z version formats in matching (WI-2026-02-25-001)
- Path parser supports segment/index grammar with tests (WI-2026-02-25-003)
- ADR set/get/add/remove accept path-based field addressing for alternatives/pros/cons/rejection_reason (WI-2026-02-25-003)
- edit-ops.json SSOT and edit-ops.schema.json are introduced with build-time validation (WI-2026-02-25-003)
- path parser enforces full-input consumption and strict grammar per ADR-0030 (WI-2026-02-25-003)
- alias and legacy prefix compatibility rules are normalized via a single spec-backed layer (WI-2026-02-25-003)
- macro or generated rule table drives get/set/add/remove path binding dispatch (WI-2026-02-25-003)
- Introduce V2 edit SSOT model and schema for fields, verbs, aliases, and legacy mappings (WI-2026-02-27-001)
- Implement winnow-based path parser and canonicalization pipeline producing typed EditPlan (WI-2026-02-27-001)
- Implement unified semantic edit engine with JsonAdapter and TomlAdapter interfaces (WI-2026-02-27-001)
- Migrate skill created at .claude/skills/migrate/SKILL.md (WI-2026-03-03-001)
- Skill covers all 5 phases from ADR-0032 (WI-2026-03-03-001)

### Changed

- Rewrite incremental rendering to parse ALL existing releases into a map (WI-2026-02-25-001)
- Use relative 'docs' path instead of absolute path for inline ref expansion (WI-2026-02-25-001)
- ADR and work docs include before/after examples for path-based edits (WI-2026-02-25-003)
- Migrate ADR and Work edit execution to V2 engine with compatibility parity tests (WI-2026-02-27-001)
- Migrate RFC and Clause edit execution to V2 engine after ADR/Work parity is verified (WI-2026-02-27-001)
- Add content_path to SSOT nested rules and code generation (WI-2026-03-01-001)
- Generic nested field operations in edit_runtime replace hand-written handlers (WI-2026-03-01-001)

### Removed

- V1 manual dispatch removed from edit.rs (macros, per-artifact nested handlers) (WI-2026-03-01-001)

### Fixed

- Inline refs in CHANGELOG expanded to absolute paths instead of relative (WI-2026-02-25-001)
- Older release sections had inline refs re-expanded on each render (WI-2026-02-25-001)
- Version matching failed for v-prefixed versions (v0.3.0 vs 0.3.0) (WI-2026-02-25-001)
- Releases not in releases.toml were discarded from CHANGELOG (WI-2026-02-25-001)
- Add semantic field descriptions to work set help (WI-2026-02-25-002)
- remove rejects mixed indexed-path and matcher-flag usage with clear usage error (WI-2026-02-25-003)
- over-deep or over-specified paths fail with structured diagnostics instead of silent acceptance (WI-2026-02-25-003)

## [0.5.2] - 2026-02-24

### Added

- test_work_add_journal integration test (WI-2026-02-24-001)

### Fixed

- work add journal command succeeds (WI-2026-02-24-001)
- snapshot tests stable across days/machines (WI-2026-02-24-001)

## [0.5.1] - 2026-02-22

### Added

- Journal section in work item render output (WI-2026-02-22-001)
- Journal field definition in work.schema.toml (WI-2026-02-22-001)
- JournalEntry struct and journal field in WorkItemContent (WI-2026-02-22-001)
- rejection_reason field to Alternative struct (WI-2026-02-22-002)
- pros and cons fields to Alternative struct (WI-2026-02-22-002)
- name field to discuss/gov/quick skill frontmatter (WI-2026-02-22-003)
- discuss/gov/quick skills in .claude/skills/ (WI-2026-02-22-003)
- commit skill with govctl integration (WI-2026-02-22-003)

### Changed

- wi-writer skill documentation with journal usage (WI-2026-02-22-001)
- ADR render output includes pros/cons/rejection_reason (WI-2026-02-22-002)
- adr.schema.toml with Alternative field documentation (WI-2026-02-22-002)
- src/cmd/new.rs to use unified SKILL_TEMPLATES (WI-2026-02-22-003)
- adr set/get/add/remove commands with valid field list (WI-2026-02-22-004)
- rfc set/get commands with valid field list (WI-2026-02-22-004)
- clause set/get commands with valid field list (WI-2026-02-22-004)
- work set/get/add/remove commands with valid field list (WI-2026-02-22-004)

### Removed

- commands/status.md due to low usage (WI-2026-02-22-003)

## [0.4.3] - 2026-02-16

### Added

- Exclusive gov-root lock acquired for all write commands before mutating gov/ or docs/ (WI-2026-02-15-001)
- Read-only commands do not acquire lock or block (WI-2026-02-15-001)
- Bounded wait with configurable or documented default (recommend >= 30s); clear error on timeout (WI-2026-02-15-001)
- Lock released on process exit; no deadlock when holder crashes (WI-2026-02-15-001)

### Changed

- commands_dir renamed to agent_dir in PathsConfig (WI-2026-02-16-002)
- Default changed from .claude/commands to .claude (WI-2026-02-16-002)
- sync_commands uses agent_dir directly (WI-2026-02-16-002)

### Fixed

- All user-facing path outputs use relative paths (gov/..., docs/...) (WI-2026-02-16-001)
- Config::display_path() converts absolute paths to project-root-relative (WI-2026-02-16-001)
- write_file, create_dir_all, delete_file have optional display_path parameter (WI-2026-02-16-001)

## [0.4.2] - 2026-02-13

### Fixed

- All warnings have concise hints (WI-2026-02-13-001)

## [0.4.1] - 2026-02-11

### Added

- Create assets/skills/ and assets/agents/ directories (WI-2026-02-11-002)
- rfc-writer skill (WI-2026-02-11-003)
- adr-writer skill (WI-2026-02-11-003)
- wi-writer skill (WI-2026-02-11-003)
- rfc-reviewer agent (WI-2026-02-11-003)
- adr-reviewer agent (WI-2026-02-11-003)
- wi-reviewer agent (WI-2026-02-11-003)
- compliance-checker agent (WI-2026-02-11-003)

### Changed

- Move command assets (gov.md, quick.md, discuss.md, status.md) into assets/commands/ (WI-2026-02-11-002)
- CLI renamed to sync with sync-commands as backward-compatible alias (WI-2026-02-11-005)

## [0.4.0] - 2026-02-07

### Added

- Implement rfc show, adr show, work show, clause show commands (WI-2026-02-07-002)
- Support -o json for structured output (WI-2026-02-07-002)
- Update describe output with show commands (WI-2026-02-07-002)

### Changed

- Rename describe --format to --output for consistency (WI-2026-02-07-003)

## [0.3.1] - 2026-02-07

### Added

- Shared header/footer with breadcrumbs and counts (WI-2026-02-07-001)
- List filter and quick-jump navigation (WI-2026-02-07-001)
- Detail views show scroll position (WI-2026-02-07-001)

### Changed

- Document TUI keymap and UX (WI-2026-02-07-001)

## [0.3.0] - 2026-01-29

### Added

- Add `govctl rfc render <RFC-ID>` command (WI-2026-01-29-003)
- Add `govctl adr render <ADR-ID>` command (WI-2026-01-29-003)
- Add `govctl work render <WI-ID>` command (WI-2026-01-29-003)

### Removed

- Remove `--rfc-id` flag from global render command (WI-2026-01-29-003)

## [0.2.4] - 2026-01-29

### Fixed

- `load_work_items()` returns items sorted by ID (WI-2026-01-29-001)
- `load_adrs()` returns items sorted by ID (WI-2026-01-29-001)
- `load_rfcs()` returns items sorted by ID (WI-2026-01-29-001)

## [0.2.3] - 2026-01-26

### Added

- ADR documents ID strategy decision (WI-2026-01-26-005)
- Config supports id_strategy field with sequential/author-hash/random options (WI-2026-01-26-005)
- author-hash strategy uses git user.email for namespace isolation (WI-2026-01-26-005)
- random strategy generates short unique suffix (WI-2026-01-26-005)
- `error[CODE]:` diagnostic format for error messages (WI-2026-01-26-006)
- `NO_COLOR` environment variable support (WI-2026-01-26-006)
- Confirmation prompts for `deprecate` and `supersede` commands (WI-2026-01-26-006)
- `--output` flag with format option (json, yaml, table) on get/list commands (WI-2026-01-26-007)
- `-o` short flag for `--output` (WI-2026-01-26-007)
- EXAMPLES section in command help text using clap after_help (WI-2026-01-26-007)

### Changed

- SCHEMA.md documents new ID formats (WI-2026-01-26-005)

### Fixed

- Down/j navigation scrolls the list view when selection moves past visible area (WI-2026-01-26-009)
- Up/k navigation scrolls the list view when selection moves above visible area (WI-2026-01-26-009)
- Selection highlight remains visible at all scroll positions (WI-2026-01-26-009)
- Users can navigate to clause detail from RFC detail view (WI-2026-01-26-010)
- Clause detail shows full clause content (ID, title, status, text) (WI-2026-01-26-010)
- j/k navigation works in RFC detail to select clauses (WI-2026-01-26-010)

## [0.2.2] - 2026-01-26

### Added

- C-REFERENCE-HIERARCHY clause defines artifact reference rules (WI-2026-01-26-002)
- --force/-f flag for full changelog regeneration (WI-2026-01-26-004)

### Changed

- clause new sets since to null (WI-2026-01-26-003)
- rfc bump fills since for pending clauses (WI-2026-01-26-003)
- Default render changelog only updates Unreleased section (WI-2026-01-26-004)

### Fixed

- RFC rendering expands inline [[...]] refs like ADR/work items (WI-2026-01-26-001)
- RFC rendering displays refs field when present (WI-2026-01-26-001)
- Missing release sections are auto-generated and inserted in correct order (WI-2026-01-26-004)

## [0.2.1] - 2026-01-25

### Added

- --category option to 'work add' command for explicit category specification (WI-2026-01-25-001)
- E0408 error when acceptance criteria lacks explicit category (WI-2026-01-25-001)

### Fixed

- Changelog inline refs (`[[RFC-NNNN]]`) now expand to markdown links (WI-2026-01-25-002)
- Link paths use configurable `docs_output` from config (WI-2026-01-25-002)

## [0.2.0] - 2026-01-19

### Added

- `govctl describe --json` outputs machine-readable command catalog (WI-2026-01-18-003)
- `govctl describe --context --json` outputs project state with suggested actions (WI-2026-01-18-003)
- Command metadata includes `when_to_use` semantic guidance (WI-2026-01-18-003)
- All existing commands covered in describe output (WI-2026-01-18-003)
- CLI command 'govctl clause delete' implemented with proper argument parsing (WI-2026-01-19-001)
- Atomically removes clause file and updates parent RFC's clauses array (WI-2026-01-19-001)
- CLI command 'work delete' implemented (WI-2026-01-19-003)
- Only queue-status work items can be deleted (WI-2026-01-19-003)
- Reference check prevents deletion if work item is referenced (WI-2026-01-19-003)
- completions command implemented (WI-2026-01-19-004)
- works for bash/zsh/fish/powershell (WI-2026-01-19-004)
- Auto-detection from ID format working (WI-2026-01-19-005)
- Optional type flags available (WI-2026-01-19-005)
- Support RFC amendments via version bumping and changelog (WI-2026-01-19-006)
- Add signature field to RfcSpec model (WI-2026-01-19-007)
- Implement govctl bump command for version bumping (WI-2026-01-19-007)
- Display asterisk indicator for amended RFCs in list output (WI-2026-01-19-007)
- All resource commands use <resource> <verb> structure per [RFC-0002:C-RESOURCE-MODEL](docs/rfc/RFC-0002.md#rfc-0002c-resource-model) (WI-2026-01-19-008)
- Canonical command pattern eliminates duplication (WI-2026-01-19-008)
- Add -n short flag to --limit on all list commands (rfc, clause, adr, work) (WI-2026-01-19-009)
- New sync-commands command to update AI IDE commands (WI-2026-01-19-010)
- Configurable commands_dir in config.toml for different AI IDEs (WI-2026-01-19-010)

### Changed

- Remove 'normative = frozen' validation constraint (WI-2026-01-19-006)
- Update .claude/CLAUDE.md to use --dry-run without -n (WI-2026-01-19-009)
- Update assets/\*.md files to use resource-first command syntax (WI-2026-01-19-010)

### Removed

- Old verb-first commands removed per [ADR-0018](docs/adr/ADR-0018.md) (WI-2026-01-19-008)
- Remove -n short flag from global --dry-run (WI-2026-01-19-009)

### Fixed

- New clauses get 'since' field set to RFC's current version (WI-2026-01-19-002)

## [0.1.0] - 2026-01-18

### Added

- Multi-line criterion:
- First condition
- Second condition (WI-2026-01-17-001)
- Test criterion (WI-2026-01-17-001)
- `new rfc "Title"` auto-generates RFC-ID (WI-2026-01-17-002)
- `new rfc --id RFC-XXXX "Title"` allows manual ID with collision check (WI-2026-01-17-002)
- `new work` generates unique IDs (no collisions) (WI-2026-01-17-002)
- `edit --stdin` works (renamed from --text-stdin) (WI-2026-01-17-002)
- `set <ID> <FIELD> --stdin` reads value from stdin (WI-2026-01-17-002)
- `new work --active "Title"` creates active work item (WI-2026-01-17-002)
- Work Item schema supports structured `acceptance_criteria` with status (WI-2026-01-17-002)
- Work Item schema supports structured `decisions` with status (WI-2026-01-17-002)
- ADR schema supports `alternatives` array with status (WI-2026-01-17-002)
- Markdown renderer shows checkboxes (pending=unchecked, done=checked, cancelled=strikethrough) (WI-2026-01-17-002)
- `tick` command updates checklist item status (WI-2026-01-17-002)
- Validation blocks move to done if pending acceptance criteria exist (WI-2026-01-17-002)
- TUI launches with govctl tui command (WI-2026-01-17-003)
- Dashboard shows RFC/ADR/Work item counts by status (WI-2026-01-17-003)
- List views for RFC/ADR/Work with j/k navigation (WI-2026-01-17-003)
- Detail views show full content (WI-2026-01-17-003)
- Feature-gated behind --features tui (WI-2026-01-17-003)
- Clause kind enum has only 'normative' and 'informative' (WI-2026-01-17-004)
- ADR status enum includes 'rejected' (WI-2026-01-17-004)
- Work Item lifecycle allows queue to cancelled transition (WI-2026-01-17-004)
- RFC status×phase rules documented in SCHEMA.md (WI-2026-01-17-004)
- govctl reject command to reject ADR proposals (WI-2026-01-17-004)
- is_valid_adr_transition allows proposed to rejected (WI-2026-01-17-004)
- Add src/signature.rs with deterministic hash computation (WI-2026-01-17-005)
- Rendered markdown includes SIGNATURE comment (WI-2026-01-17-005)
- govctl check validates signatures (WI-2026-01-17-005)
- SCHEMA.md documents signature format (WI-2026-01-17-005)
- ADR refs field validates that referenced artifacts exist (WI-2026-01-17-006)
- Work Item refs field validates that referenced artifacts exist (WI-2026-01-17-006)
- govctl check reports diagnostics for invalid refs (WI-2026-01-17-006)
- Refs in ADRs render as markdown links (WI-2026-01-17-007)
- Refs in Work Items render as markdown links (WI-2026-01-17-007)
- Clause refs link to RFC file with anchor (WI-2026-01-17-007)
- ChangelogEntry model uses Keep a Changelog categories (WI-2026-01-17-008)
- Changelog renders with Added/Changed/Fixed/etc sections (WI-2026-01-17-008)
- RFC-0000 migrated to new format (WI-2026-01-17-008)
- Create ui module with color helpers (WI-2026-01-17-009)
- Success messages display in green (WI-2026-01-17-009)
- Error messages display in red (WI-2026-01-17-009)
- File paths and IDs highlighted (WI-2026-01-17-009)
- Add insta and insta-cmd as dev dependencies (WI-2026-01-17-015)
- Create minimal test fixtures for valid and invalid governance states (WI-2026-01-17-015)
- Implement test harness with path normalization and color stripping (WI-2026-01-17-015)
- Add snapshot tests for check, list, and status commands (WI-2026-01-17-015)
- Remove decisions field from WorkItemContent struct (WI-2026-01-17-017)
- Change notes field from String to `Vec<String>` (WI-2026-01-17-017)
- Migrate existing work items to new schema (WI-2026-01-17-017)
- Update new work item template (WI-2026-01-17-017)
- All existing tests pass (WI-2026-01-17-018)
- Move to done rejects if acceptance_criteria is empty (WI-2026-01-17-018)
- Error message suggests how to add criteria (WI-2026-01-17-018)
- README only references RFCs that actually exist (WI-2026-01-17-019)
- Add concrete Before/After example (WI-2026-01-17-020)
- Add visual workflow diagram (WI-2026-01-17-020)
- Add Who This Is For section (WI-2026-01-17-020)
- Soften Contributing section tone (WI-2026-01-17-020)
- Fix tick command syntax (use -s flag) (WI-2026-01-17-021)
- Fix work item paths (gov/work/ not worklogs/items/) (WI-2026-01-17-021)
- Fix new rfc syntax (--id flag) (WI-2026-01-17-021)
- Fix edit command (--stdin not --text) (WI-2026-01-17-021)
- Add fast path for doc-only changes (WI-2026-01-17-021)
- Add --active flag for new work (WI-2026-01-17-021)
- Reduce multi-line input section verbosity (WI-2026-01-17-021)
- Add acceptance criteria setup in Phase 1 (WI-2026-01-17-021)
- Fix tick command syntax to use -s flag (WI-2026-01-17-022)
- Add refs field to RfcSpec model (WI-2026-01-17-023)
- Validate RFC refs against known artifacts (WI-2026-01-17-023)
- Add diagnostic code for RFC ref not found (WI-2026-01-17-023)
- Support add/remove/get refs for RFCs in edit commands (WI-2026-01-17-023)
- Add SourceScanConfig to config.rs with enabled, roots, exts, pattern fields (WI-2026-01-17-024)
- Add E0107SourceRefUnknown and W0107SourceRefOutdated diagnostic codes (WI-2026-01-17-024)
- Create scan.rs with scan_source_refs function (WI-2026-01-17-024)
- Integrate scanner into check command (WI-2026-01-17-024)
- Add warning diagnostic for placeholder descriptions (WI-2026-01-17-025)
- Detect common placeholder patterns (WI-2026-01-17-025)
- All work items have meaningful descriptions (WI-2026-01-17-026)
- No W0108 warnings from govctl check (WI-2026-01-17-026)
- Add `expand_inline_refs()` function using source_scan pattern (WI-2026-01-17-027)
- Apply expansion to ADR content fields (context, decision, consequences) (WI-2026-01-17-027)
- Apply expansion to work item content fields (description, notes, acceptance_criteria) (WI-2026-01-17-027)
- Parse prefix from change string (add:, fix:, changed:, deprecated:, removed:, security:) (WI-2026-01-17-028)
- Route changes to correct changelog category based on prefix (WI-2026-01-17-028)
- Validate unknown prefixes with helpful error message (WI-2026-01-17-028)
- Default to added category when no prefix (WI-2026-01-17-028)
- Add `category` field to `ChecklistItem` with default `Added` (WI-2026-01-17-029)
- Update `add acceptance_criteria` command to parse prefixes per [ADR-0012](docs/adr/ADR-0012.md) (WI-2026-01-17-029)
- `Release` and `ReleasesFile` models in model.rs with version, date, and refs fields (WI-2026-01-17-030)
- Load and save `gov/releases.toml` functionality with semver validation (WI-2026-01-17-030)
- `govctl release <version>` command to cut releases (WI-2026-01-17-030)
- `govctl render changelog` command with Keep a Changelog format (WI-2026-01-17-030)
- New E08xx diagnostic codes for CLI/command errors (WI-2026-01-18-001)
- Extended E01xx-E04xx codes for lifecycle operations (WI-2026-01-18-001)

### Changed

- Consolidate `ChangelogCategory` enum into `model.rs` with `from_prefix` method (WI-2026-01-17-029)
- Update `work.schema.toml` with category field documentation (WI-2026-01-17-029)
- Update work template with category field documentation (WI-2026-01-17-029)
- All anyhow::bail! calls converted to Diagnostic errors (WI-2026-01-18-001)

### Fixed

- All existing work items remain valid (backward compatible default) (WI-2026-01-17-029)
- mv command help shows FILE_OR_ID instead of FILE (WI-2026-01-18-002)
