//! Describe command implementation - machine-readable CLI metadata for agents.

mod catalog;
mod context;

use crate::cmd::output::print_json;
use crate::config::Config;
use crate::diagnostic::{DiagnosticCode, DiagnosticResult, Diagnostics};
use serde::Serialize;

use catalog::{CommandInfo, WorkflowInfo, command_catalog, workflow_info};
use context::{ProjectState, load_context};

/// Versioned wire contract defined by [[RFC-0002:C-DESCRIBE-COMMAND]].
#[derive(Serialize)]
struct DescribeOutput {
    schema_version: u32,
    tool_version: String,
    purpose: String,
    commands: Vec<CommandInfo>,
    workflow: WorkflowInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_state: Option<ProjectState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_actions: Option<Vec<String>>,
}

/// Execute describe command
pub fn describe(config: &Config, include_context: bool) -> DiagnosticResult<Diagnostics> {
    let mut output = DescribeOutput {
        schema_version: 1,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        purpose: "Governance-as-code CLI for AI coding agents".to_string(),
        commands: command_catalog(),
        workflow: workflow_info(),
        project_state: None,
        suggested_actions: None,
    };

    if include_context {
        let context = match load_context(config) {
            Ok(context) => context,
            Err(diagnostics) => return Ok(diagnostics),
        };
        output.project_state = Some(context.project_state);
        output.suggested_actions = Some(context.suggested_actions);
    }

    print_json(
        &output,
        DiagnosticCode::E0903UnexpectedError,
        "Failed to serialize command description",
        "describe",
    )?;

    Ok(vec![])
}
