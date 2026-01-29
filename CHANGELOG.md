# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
