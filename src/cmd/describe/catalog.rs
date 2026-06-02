use serde::Serialize;

#[derive(Serialize)]
pub struct CommandInfo {
    pub name: String,
    pub purpose: String,
    pub when_to_use: String,
    pub example: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<String>,
}

#[derive(Serialize)]
pub struct WorkflowInfo {
    pub phases: Vec<String>,
    pub typical_sequence: Vec<String>,
}

fn command(
    name: &str,
    purpose: &str,
    when_to_use: &str,
    example: &str,
    prerequisites: &[&str],
) -> CommandInfo {
    CommandInfo {
        name: name.to_string(),
        purpose: purpose.to_string(),
        when_to_use: when_to_use.to_string(),
        example: example.to_string(),
        prerequisites: prerequisites.iter().map(|item| item.to_string()).collect(),
    }
}

/// Get static command metadata
pub(super) fn command_catalog() -> Vec<CommandInfo> {
    vec![
        command(
            "init",
            "Initialize govctl governance structure in the current directory",
            "Once per project, before any other govctl commands. Creates gov/ directory structure, config, and schemas.",
            "govctl init",
            &[],
        ),
        command(
            "init-skills",
            "Install agent skills and agents into the project",
            "After govctl init, if not using the govctl plugin. Installs skills and agents into the configured agent_dir.",
            "govctl init-skills",
            &["govctl init"],
        ),
        command(
            "status",
            "Show summary counts of all artifacts",
            "To get an overview of project governance state. Run at start of session to understand current work.",
            "govctl status",
            &["govctl init"],
        ),
        command(
            "check",
            "Validate all governed documents",
            "Before committing, after edits, to verify governance compliance. Run frequently during development.",
            "govctl check",
            &["govctl init"],
        ),
        command(
            "verify",
            "Run reusable verification guards",
            "To execute project-level or work-item-specific completion gates before marking work done.",
            "govctl verify --work WI-2026-01-18-001",
            &["govctl init"],
        ),
        command(
            "loop list",
            "List persisted local loop states",
            "To discover interrupted or resumable loops before selecting one by loop ID.",
            "govctl loop list open",
            &["govctl init"],
        ),
        command(
            "loop start",
            "Start or reuse a loop for one or more explicit work items",
            "When beginning a local execution loop for a work item or batch of work items.",
            "govctl loop start WI-2026-04-06-001",
            &["govctl init"],
        ),
        command(
            "loop show",
            "Show persisted loop state",
            "To inspect loop state, resolved work items, dependencies, and round counts by loop ID.",
            "govctl loop show LOOP-2026-04-06-001",
            &["Loop must exist"],
        ),
        command(
            "loop resume",
            "Resume or inspect an existing non-terminal loop",
            "After discovering a loop with loop list, use its loop ID to resume local execution state.",
            "govctl loop resume LOOP-2026-04-06-001",
            &["Loop must exist"],
        ),
        command(
            "loop replan",
            "Recompute dependency closure for a loop's current work set",
            "After work item dependency files change and the existing loop needs a refreshed resolved plan.",
            "govctl loop replan LOOP-2026-04-06-001",
            &["Loop must exist"],
        ),
        command(
            "loop add",
            "Add a work item to a loop's editable work field",
            "To expand an existing loop without creating a new loop ID.",
            "govctl loop add LOOP-2026-04-06-001 work WI-2026-04-06-002",
            &["Loop must exist", "Work item must exist"],
        ),
        command(
            "loop remove",
            "Remove a work item from a loop's editable work field",
            "To narrow an existing loop while preserving completed work item lifecycle changes.",
            "govctl loop remove LOOP-2026-04-06-001 work WI-2026-04-06-002",
            &["Loop must exist"],
        ),
        command(
            "loop run",
            "Run execution rounds for an existing loop",
            "To execute ready work items in a loop by loop ID, optionally targeting specific work items.",
            "govctl loop run LOOP-2026-04-06-001 --work WI-2026-04-06-002",
            &["Loop must exist"],
        ),
        command(
            "rfc list",
            "List all RFCs with their status and phase",
            "To see all specifications. Filter by status: 'govctl rfc list draft'.",
            "govctl rfc list",
            &["govctl init"],
        ),
        command(
            "adr list",
            "List all ADRs (Architecture Decision Records)",
            "To see architectural decisions. Filter by status: 'govctl adr list accepted'.",
            "govctl adr list",
            &["govctl init"],
        ),
        command(
            "work list",
            "List work items (defaults to pending: queue + active)",
            "To see current task queue. Use 'govctl work list all' for everything.",
            "govctl work list",
            &["govctl init"],
        ),
        command(
            "rfc new",
            "Create a new RFC (specification document)",
            "Before implementing any new feature. RFCs define what must be built. No implementation without specification.",
            "govctl rfc new \"Add caching layer\"",
            &["govctl init"],
        ),
        command(
            "adr new",
            "Create a new ADR (Architecture Decision Record)",
            "When making a significant design decision that should be documented. ADRs capture context, decision, and consequences.",
            "govctl adr new \"Use Redis for caching\"",
            &["govctl init"],
        ),
        command(
            "work new",
            "Create a new work item",
            "When starting a task. Use --active to immediately activate it.",
            "govctl work new \"Implement describe command\" --active",
            &["govctl init"],
        ),
        command(
            "clause new",
            "Create a new clause within an RFC",
            "When adding normative requirements to an RFC. Clauses are the atomic units of specification.",
            "govctl clause new RFC-0001:C-CACHE-TTL \"Cache TTL Policy\" -s Specification -k normative",
            &["RFC must exist"],
        ),
        command(
            "rfc finalize",
            "Transition RFC status to normative or deprecated",
            "When an RFC spec is complete and ready for implementation. 'normative' makes it binding law.",
            "govctl rfc finalize RFC-0001 normative",
            &["RFC must be in draft status"],
        ),
        command(
            "rfc advance",
            "Advance RFC phase (spec → impl → test → stable)",
            "After completing work for current phase. Phase discipline ensures proper workflow.",
            "govctl rfc advance RFC-0001 impl",
            &["RFC should be normative", "Current phase work complete"],
        ),
        command(
            "work move",
            "Move work item to new status (queue/active/done/cancelled)",
            "To update task status. Use 'done' when complete, 'active' to start working.",
            "govctl work move WI-2026-01-18-001 done",
            &[
                "Work item must exist",
                "For 'done': acceptance criteria required",
            ],
        ),
        command(
            "adr accept",
            "Accept an ADR (proposed → accepted)",
            "When an architectural decision is approved.",
            "govctl adr accept ADR-0001",
            &["ADR must be in proposed status"],
        ),
        command(
            "rfc set / adr set / work set / guard set / clause set",
            "Set a field value on an artifact",
            "To update artifact fields. Use --stdin for multi-line content.",
            "govctl rfc set RFC-0001 title \"New Title\"",
            &["Artifact must exist"],
        ),
        command(
            "rfc get / adr get / work get / guard get / clause get",
            "Get a field value from an artifact",
            "To read artifact data. Omit field name to show entire artifact.",
            "govctl rfc get RFC-0001 status",
            &["Artifact must exist"],
        ),
        command(
            "rfc add / adr add / work add / guard add",
            "Add a value to an array field",
            "To add items to refs, owners, acceptance_criteria, etc.",
            "govctl work add WI-2026-01-18-001 acceptance_criteria \"Tests pass\"",
            &["Artifact must exist"],
        ),
        command(
            "rfc remove / adr remove / work remove / guard remove",
            "Remove a value from an array field",
            "To remove items from array fields. Use --at for index, or pattern matching.",
            "govctl rfc remove RFC-0001 owners \"@oldowner\"",
            &["Artifact must exist"],
        ),
        command(
            "work tick / adr tick",
            "Mark a checklist item as done/pending/cancelled",
            "To update acceptance criteria status on work items.",
            "govctl work tick WI-2026-01-18-001 acceptance_criteria \"Tests\" -s done",
            &["Work item or ADR must exist"],
        ),
        command(
            "rfc edit / adr edit / work edit / guard edit / clause edit",
            "Edit artifact fields via the canonical path-first surface",
            "To update RFC, ADR, work item, guard, or clause content fields using `edit <ID> <path> --set/--add/--remove/--tick`.",
            "govctl clause edit RFC-0001:C-SCOPE text --stdin",
            &["Target artifact must exist"],
        ),
        command(
            "render",
            "Render artifacts to markdown",
            "To generate human-readable documentation from SSOT. Run after RFC changes.",
            "govctl render rfc",
            &["govctl init"],
        ),
        command(
            "migrate",
            "Convert legacy JSON governance storage to current TOML formats",
            "When a repository still stores RFCs or clauses as JSON, or when releases.toml needs schema metadata normalization.",
            "govctl migrate",
            &["govctl init"],
        ),
        command(
            "rfc bump",
            "Bump RFC version",
            "When making changes to a normative RFC. Follows semver.",
            "govctl rfc bump RFC-0001 --minor -m \"Add new clause\"",
            &["RFC must exist"],
        ),
        command(
            "release",
            "Cut a release (collect unreleased work items)",
            "When releasing a new version. Collects done work items into changelog.",
            "govctl release 0.2.0",
            &["Done work items exist"],
        ),
        command(
            "rfc deprecate / clause deprecate",
            "Deprecate an artifact",
            "When an RFC or clause is no longer relevant but kept for history.",
            "govctl rfc deprecate RFC-0001",
            &["Artifact must exist"],
        ),
        command(
            "rfc supersede / adr supersede / clause supersede",
            "Supersede an artifact with a replacement",
            "When replacing an artifact with a newer version.",
            "govctl rfc supersede RFC-0001 --by RFC-0010",
            &["Both artifacts must exist"],
        ),
        command(
            "rfc show",
            "Show RFC content to stdout (no file written)",
            "To read the full rendered RFC content. Use -o json for structured output.",
            "govctl rfc show RFC-0001",
            &["RFC must exist"],
        ),
        command(
            "adr show",
            "Show ADR content to stdout (no file written)",
            "To read the full rendered ADR content. Use -o json for structured output.",
            "govctl adr show ADR-0001",
            &["ADR must exist"],
        ),
        command(
            "work show",
            "Show work item content to stdout (no file written)",
            "To read the full rendered work item content. Use -o json for structured output.",
            "govctl work show WI-2026-01-18-001",
            &["Work item must exist"],
        ),
        command(
            "clause show",
            "Show clause content to stdout (no file written)",
            "To read the clause text. Use -o json for structured output.",
            "govctl clause show RFC-0001:C-SUMMARY",
            &["Clause must exist"],
        ),
    ]
}

/// Get workflow info
pub(super) fn workflow_info() -> WorkflowInfo {
    WorkflowInfo {
        phases: vec![
            "spec: RFC drafting and design discussion".to_string(),
            "impl: Code writing per normative RFC".to_string(),
            "test: Verification and test writing".to_string(),
            "stable: Bug fixes only, no new features".to_string(),
        ],
        typical_sequence: vec![
            "govctl work new \"Feature Title\" --active".to_string(),
            "govctl rfc new \"Feature Title\"".to_string(),
            "govctl clause new RFC-NNNN:C-REQUIREMENT \"Requirement\" -k normative".to_string(),
            "govctl rfc finalize RFC-NNNN normative".to_string(),
            "govctl rfc advance RFC-NNNN impl".to_string(),
            "# Implement the feature".to_string(),
            "govctl rfc advance RFC-NNNN test".to_string(),
            "# Write tests".to_string(),
            "govctl rfc advance RFC-NNNN stable".to_string(),
            "govctl work tick WI-xxx acceptance_criteria \"criterion\" -s done".to_string(),
            "govctl work move WI-xxx done".to_string(),
        ],
    }
}
