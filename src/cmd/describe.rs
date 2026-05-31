//! Describe command implementation - machine-readable CLI metadata for agents.

mod catalog;
mod context;

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use serde::Serialize;

use catalog::{CommandInfo, WorkflowInfo, command_catalog, workflow_info};
use context::{ProjectState, SuggestedAction, load_context};

/// Output format for describe command
#[derive(Serialize)]
struct DescribeOutput {
    version: String,
    purpose: String,
    philosophy: Vec<String>,
    commands: Vec<CommandInfo>,
    workflow: WorkflowInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_state: Option<ProjectState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_actions: Option<Vec<SuggestedAction>>,
}

/// Execute describe command
pub fn describe(config: &Config, include_context: bool) -> anyhow::Result<Vec<Diagnostic>> {
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

    if include_context && let Some(context) = load_context(config) {
        output.project_state = Some(context.project_state);
        output.suggested_actions = Some(context.suggested_actions);
    }

    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);

    Ok(vec![])
}
