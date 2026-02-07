//! Describe command implementation - machine-readable CLI metadata for agents.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use serde::Serialize;

/// Output format for describe command
#[derive(Serialize)]
pub struct DescribeOutput {
    pub version: String,
    pub purpose: String,
    pub philosophy: Vec<String>,
    pub commands: Vec<CommandInfo>,
    pub workflow: WorkflowInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_state: Option<ProjectState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_actions: Option<Vec<SuggestedAction>>,
}

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

#[derive(Serialize)]
pub struct ProjectState {
    pub rfcs: Vec<RfcState>,
    pub adrs: Vec<AdrState>,
    pub work_items: Vec<WorkItemState>,
}

#[derive(Serialize)]
pub struct RfcState {
    pub id: String,
    pub title: String,
    pub status: String,
    pub phase: String,
}

#[derive(Serialize)]
pub struct AdrState {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct WorkItemState {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct SuggestedAction {
    pub command: String,
    pub reason: String,
    pub priority: String,
}

/// Get static command metadata
fn command_catalog() -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            name: "init".to_string(),
            purpose: "Initialize govctl in the current directory".to_string(),
            when_to_use: "Once per project, before any other govctl commands. Creates gov/ directory structure and config.".to_string(),
            example: "govctl init".to_string(),
            prerequisites: vec![],
        },
        CommandInfo {
            name: "status".to_string(),
            purpose: "Show summary counts of all artifacts".to_string(),
            when_to_use: "To get an overview of project governance state. Run at start of session to understand current work.".to_string(),
            example: "govctl status".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "check".to_string(),
            purpose: "Validate all governed documents".to_string(),
            when_to_use: "Before committing, after edits, to verify governance compliance. Run frequently during development.".to_string(),
            example: "govctl check".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "list rfc".to_string(),
            purpose: "List all RFCs with their status and phase".to_string(),
            when_to_use: "To see all specifications. Filter by status: 'govctl rfc list draft'.".to_string(),
            example: "govctl rfc list".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "list adr".to_string(),
            purpose: "List all ADRs (Architecture Decision Records)".to_string(),
            when_to_use: "To see architectural decisions. Filter by status: 'govctl adr list accepted'.".to_string(),
            example: "govctl adr list".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "list work".to_string(),
            purpose: "List work items (defaults to pending: queue + active)".to_string(),
            when_to_use: "To see current task queue. Use 'govctl work list all' for everything.".to_string(),
            example: "govctl work list".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "new rfc".to_string(),
            purpose: "Create a new RFC (specification document)".to_string(),
            when_to_use: "Before implementing any new feature. RFCs define what must be built. No implementation without specification.".to_string(),
            example: "govctl rfc new \"Add caching layer\"".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "new adr".to_string(),
            purpose: "Create a new ADR (Architecture Decision Record)".to_string(),
            when_to_use: "When making a significant design decision that should be documented. ADRs capture context, decision, and consequences.".to_string(),
            example: "govctl adr new \"Use Redis for caching\"".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "new work".to_string(),
            purpose: "Create a new work item".to_string(),
            when_to_use: "When starting a task. Use --active to immediately activate it.".to_string(),
            example: "govctl work new --active \"Implement describe command\"".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "new clause".to_string(),
            purpose: "Create a new clause within an RFC".to_string(),
            when_to_use: "When adding normative requirements to an RFC. Clauses are the atomic units of specification.".to_string(),
            example: "govctl clause new RFC-0001:C-CACHE-TTL \"Cache TTL Policy\" -s Specification -k normative".to_string(),
            prerequisites: vec!["RFC must exist".to_string()],
        },
        CommandInfo {
            name: "finalize".to_string(),
            purpose: "Transition RFC status to normative or deprecated".to_string(),
            when_to_use: "When an RFC spec is complete and ready for implementation. 'normative' makes it binding law.".to_string(),
            example: "govctl rfc finalize RFC-0001 normative".to_string(),
            prerequisites: vec!["RFC must be in draft status".to_string()],
        },
        CommandInfo {
            name: "advance".to_string(),
            purpose: "Advance RFC phase (spec → impl → test → stable)".to_string(),
            when_to_use: "After completing work for current phase. Phase discipline ensures proper workflow.".to_string(),
            example: "govctl rfc advance RFC-0001 impl".to_string(),
            prerequisites: vec!["RFC should be normative".to_string(), "Current phase work complete".to_string()],
        },
        CommandInfo {
            name: "move".to_string(),
            purpose: "Move work item to new status (queue/active/done/cancelled)".to_string(),
            when_to_use: "To update task status. Use 'done' when complete, 'active' to start working.".to_string(),
            example: "govctl work move WI-2026-01-18-001 done".to_string(),
            prerequisites: vec!["Work item must exist".to_string(), "For 'done': acceptance criteria required".to_string()],
        },
        CommandInfo {
            name: "accept".to_string(),
            purpose: "Accept an ADR (proposed → accepted)".to_string(),
            when_to_use: "When an architectural decision is approved.".to_string(),
            example: "govctl adr accept ADR-0001".to_string(),
            prerequisites: vec!["ADR must be in proposed status".to_string()],
        },
        CommandInfo {
            name: "set".to_string(),
            purpose: "Set a field value on an artifact".to_string(),
            when_to_use: "To update artifact fields. Use --stdin for multi-line content.".to_string(),
            example: "govctl rfc set RFC-0001 title \"New Title\"".to_string(),
            prerequisites: vec!["Artifact must exist".to_string()],
        },
        CommandInfo {
            name: "get".to_string(),
            purpose: "Get a field value from an artifact".to_string(),
            when_to_use: "To read artifact data. Omit field name to show entire artifact.".to_string(),
            example: "govctl rfc get RFC-0001 status".to_string(),
            prerequisites: vec!["Artifact must exist".to_string()],
        },
        CommandInfo {
            name: "add".to_string(),
            purpose: "Add a value to an array field".to_string(),
            when_to_use: "To add items to refs, owners, acceptance_criteria, etc.".to_string(),
            example: "govctl work add WI-2026-01-18-001 acceptance_criteria \"Tests pass\"".to_string(),
            prerequisites: vec!["Artifact must exist".to_string()],
        },
        CommandInfo {
            name: "remove".to_string(),
            purpose: "Remove a value from an array field".to_string(),
            when_to_use: "To remove items from array fields. Use --at for index, or pattern matching.".to_string(),
            example: "govctl rfc remove RFC-0001 owners \"@oldowner\"".to_string(),
            prerequisites: vec!["Artifact must exist".to_string()],
        },
        CommandInfo {
            name: "tick".to_string(),
            purpose: "Mark a checklist item as done/pending/cancelled".to_string(),
            when_to_use: "To update acceptance criteria status on work items.".to_string(),
            example: "govctl work tick WI-2026-01-18-001 acceptance_criteria \"Tests\" -s done".to_string(),
            prerequisites: vec!["Work item or ADR must exist".to_string()],
        },
        CommandInfo {
            name: "edit".to_string(),
            purpose: "Edit clause text".to_string(),
            when_to_use: "To update normative clause content. Use --stdin for multi-line text.".to_string(),
            example: "govctl clause edit RFC-0001:C-SCOPE --stdin".to_string(),
            prerequisites: vec!["Clause must exist".to_string()],
        },
        CommandInfo {
            name: "render".to_string(),
            purpose: "Render artifacts to markdown".to_string(),
            when_to_use: "To generate human-readable documentation from SSOT. Run after RFC changes.".to_string(),
            example: "govctl render rfc".to_string(),
            prerequisites: vec!["govctl init".to_string()],
        },
        CommandInfo {
            name: "bump".to_string(),
            purpose: "Bump RFC version".to_string(),
            when_to_use: "When making changes to a normative RFC. Follows semver.".to_string(),
            example: "govctl rfc bump RFC-0001 --minor -m \"Add new clause\"".to_string(),
            prerequisites: vec!["RFC must exist".to_string()],
        },
        CommandInfo {
            name: "release".to_string(),
            purpose: "Cut a release (collect unreleased work items)".to_string(),
            when_to_use: "When releasing a new version. Collects done work items into changelog.".to_string(),
            example: "govctl release 0.2.0".to_string(),
            prerequisites: vec!["Done work items exist".to_string()],
        },
        CommandInfo {
            name: "deprecate".to_string(),
            purpose: "Deprecate an artifact".to_string(),
            when_to_use: "When an RFC, clause, or ADR is no longer relevant but kept for history.".to_string(),
            example: "govctl rfc deprecate RFC-0001".to_string(),
            prerequisites: vec!["Artifact must exist".to_string()],
        },
        CommandInfo {
            name: "supersede".to_string(),
            purpose: "Supersede an artifact with a replacement".to_string(),
            when_to_use: "When replacing an artifact with a newer version.".to_string(),
            example: "govctl rfc supersede RFC-0001 --by RFC-0010".to_string(),
            prerequisites: vec!["Both artifacts must exist".to_string()],
        },
        CommandInfo {
            name: "show rfc".to_string(),
            purpose: "Show RFC content to stdout (no file written)".to_string(),
            when_to_use: "To read the full rendered RFC content. Use -o json for structured output.".to_string(),
            example: "govctl rfc show RFC-0001".to_string(),
            prerequisites: vec!["RFC must exist".to_string()],
        },
        CommandInfo {
            name: "show adr".to_string(),
            purpose: "Show ADR content to stdout (no file written)".to_string(),
            when_to_use: "To read the full rendered ADR content. Use -o json for structured output.".to_string(),
            example: "govctl adr show ADR-0001".to_string(),
            prerequisites: vec!["ADR must exist".to_string()],
        },
        CommandInfo {
            name: "show work".to_string(),
            purpose: "Show work item content to stdout (no file written)".to_string(),
            when_to_use: "To read the full rendered work item content. Use -o json for structured output.".to_string(),
            example: "govctl work show WI-2026-01-18-001".to_string(),
            prerequisites: vec!["Work item must exist".to_string()],
        },
        CommandInfo {
            name: "show clause".to_string(),
            purpose: "Show clause content to stdout (no file written)".to_string(),
            when_to_use: "To read the clause text. Use -o json for structured output.".to_string(),
            example: "govctl clause show RFC-0001:C-SUMMARY".to_string(),
            prerequisites: vec!["Clause must exist".to_string()],
        },
    ]
}

/// Get workflow info
fn workflow_info() -> WorkflowInfo {
    WorkflowInfo {
        phases: vec![
            "spec: RFC drafting and design discussion".to_string(),
            "impl: Code writing per normative RFC".to_string(),
            "test: Verification and test writing".to_string(),
            "stable: Bug fixes only, no new features".to_string(),
        ],
        typical_sequence: vec![
            "govctl work new --active \"Feature Title\"".to_string(),
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

/// Generate suggested actions based on project state
fn generate_suggestions(
    rfcs: &[RfcState],
    adrs: &[AdrState],
    work_items: &[WorkItemState],
) -> Vec<SuggestedAction> {
    let mut suggestions = Vec::new();

    // Check for draft RFCs that might be ready to finalize
    for rfc in rfcs {
        if rfc.status == "draft" {
            suggestions.push(SuggestedAction {
                command: format!("govctl rfc finalize {} normative", rfc.id),
                reason: format!(
                    "{} is in draft status. If the spec is complete, finalize it to make it binding.",
                    rfc.id
                ),
                priority: "medium".to_string(),
            });
        }

        // Check for RFCs that can advance
        match (rfc.status.as_str(), rfc.phase.as_str()) {
            ("normative", "spec") => {
                suggestions.push(SuggestedAction {
                    command: format!("govctl rfc advance {} impl", rfc.id),
                    reason: format!(
                        "{} is normative but still in spec phase. Advance to impl when ready to implement.",
                        rfc.id
                    ),
                    priority: "high".to_string(),
                });
            }
            ("normative", "impl") => {
                suggestions.push(SuggestedAction {
                    command: format!("govctl rfc advance {} test", rfc.id),
                    reason: format!(
                        "{} is in impl phase. Advance to test when implementation is complete.",
                        rfc.id
                    ),
                    priority: "medium".to_string(),
                });
            }
            ("normative", "test") => {
                suggestions.push(SuggestedAction {
                    command: format!("govctl rfc advance {} stable", rfc.id),
                    reason: format!(
                        "{} is in test phase. Advance to stable when tests pass.",
                        rfc.id
                    ),
                    priority: "medium".to_string(),
                });
            }
            _ => {}
        }
    }

    // Check for proposed ADRs
    for adr in adrs {
        if adr.status == "proposed" {
            suggestions.push(SuggestedAction {
                command: format!("govctl adr accept {}", adr.id),
                reason: format!(
                    "{} is proposed. Accept it if the decision is approved.",
                    adr.id
                ),
                priority: "medium".to_string(),
            });
        }
    }

    // Check for active work items
    let active_count = work_items.iter().filter(|w| w.status == "active").count();
    let queue_count = work_items.iter().filter(|w| w.status == "queue").count();

    if active_count == 0 && queue_count > 0 {
        suggestions.push(SuggestedAction {
            command: "govctl work list queue".to_string(),
            reason: format!(
                "No active work items but {} in queue. Consider activating one.",
                queue_count
            ),
            priority: "high".to_string(),
        });
    }

    for work_item in work_items {
        if work_item.status == "active" {
            suggestions.push(SuggestedAction {
                command: format!("govctl work move {} done", work_item.id),
                reason: format!(
                    "{} is active. Mark it done when acceptance criteria are met.",
                    work_item.id
                ),
                priority: "low".to_string(),
            });
        }
    }

    suggestions
}

/// Execute describe command
pub fn describe(config: &Config, context: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    let mut output = DescribeOutput {
        version,
        purpose: "Enforces RFC-driven phase discipline for AI-assisted software development"
            .to_string(),
        philosophy: vec![
            "RFC is the source of truth — No implementation without specification".to_string(),
            "Phases are enforced — spec → impl → test → stable".to_string(),
            "Governance is executable — Rules are checked, not suggested".to_string(),
        ],
        commands: command_catalog(),
        workflow: workflow_info(),
        project_state: None,
        suggested_actions: None,
    };

    // Add context-aware information if requested
    if context && let Ok(index) = load_project(config) {
        let rfcs: Vec<RfcState> = index
            .rfcs
            .iter()
            .map(|r| RfcState {
                id: r.rfc.rfc_id.clone(),
                title: r.rfc.title.clone(),
                status: r.rfc.status.as_ref().to_string(),
                phase: r.rfc.phase.as_ref().to_string(),
            })
            .collect();

        let adrs: Vec<AdrState> = index
            .adrs
            .iter()
            .map(|a| AdrState {
                id: a.meta().id.clone(),
                title: a.meta().title.clone(),
                status: a.meta().status.as_ref().to_string(),
            })
            .collect();

        let work_items: Vec<WorkItemState> = index
            .work_items
            .iter()
            .map(|w| WorkItemState {
                id: w.meta().id.clone(),
                title: w.meta().title.clone(),
                status: w.meta().status.as_ref().to_string(),
            })
            .collect();

        let suggestions = generate_suggestions(&rfcs, &adrs, &work_items);

        output.project_state = Some(ProjectState {
            rfcs,
            adrs,
            work_items,
        });
        output.suggested_actions = Some(suggestions);
    }

    // Output as JSON
    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);

    Ok(vec![])
}
