//! Native-client hook protocol adapters.
//!
//! Implements the bundled hook behavior in [[RFC-0002:C-AGENT-INTEGRATION]].

use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::AgentHookEvent;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::loop_state::LoopLifecycleState;
use crate::model::WorkItemStatus;

const HOOK_ITEM_LIMIT: usize = 8;
const HOOK_CONTEXT_CHAR_LIMIT: usize = 4_000;
const DIRECT_EDIT_ADVISORY: &str = "This edit targets a lifecycle-managed govctl artifact. \
Prefer the resource-specific `govctl ... edit` or lifecycle command when it can express the \
change. Direct editing is permitted when the current CLI has no suitable operation; preserve \
lifecycle-owned fields and run `govctl check` after the edit.";

#[derive(Deserialize)]
struct HookInput {
    cwd: PathBuf,
    #[serde(default)]
    tool_input: Value,
}

pub fn handle_hook(event: AgentHookEvent) -> DiagnosticResult<Diagnostics> {
    let input = read_hook_input()?;
    match event {
        AgentHookEvent::SessionStart => handle_session_start(&input),
        AgentHookEvent::PreToolUse => handle_pre_tool_use(&input),
    }
}

fn read_hook_input() -> DiagnosticResult<HookInput> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| hook_error(format!("Cannot read hook input: {err}")))?;
    let mut input: HookInput = serde_json::from_str(&input)
        .map_err(|err| hook_error(format!("Invalid native-client hook input: {err}")))?;
    if !input.cwd.is_absolute() {
        let current = std::env::current_dir()
            .map_err(|err| hook_error(format!("Cannot resolve hook working directory: {err}")))?;
        input.cwd = current.join(&input.cwd);
    }
    input.cwd = normalize_path(&input.cwd);
    Ok(input)
}

fn handle_session_start(input: &HookInput) -> DiagnosticResult<Diagnostics> {
    let context = match Config::discover_from(&input.cwd) {
        Ok(None) => return Ok(vec![]),
        Ok(Some(config)) => match session_context(&config) {
            Ok(context) => context,
            Err(diagnostic) => recovery_context(&diagnostic),
        },
        Err(diagnostic) => recovery_context(&diagnostic),
    };
    emit_hook_context(AgentHookEvent::SessionStart, &context)
}

fn session_context(config: &Config) -> DiagnosticResult<String> {
    crate::load::reject_unmigrated_conformance(config)?;
    let work_result = crate::parse::load_work_items_with_warnings(config)?;
    if let Some(diagnostic) = work_result.warnings.into_iter().next() {
        return Err(diagnostic);
    }
    let mut active_work = work_result
        .items
        .into_iter()
        .filter(|work| work.meta().status == WorkItemStatus::Active)
        .collect::<Vec<_>>();
    active_work.sort_by(|left, right| left.meta().id.cmp(&right.meta().id));

    let mut loops = crate::cmd::loop_cmd::load_loop_states(config)?
        .into_iter()
        .filter(|state| {
            matches!(
                state.loop_meta.state,
                LoopLifecycleState::Pending
                    | LoopLifecycleState::Active
                    | LoopLifecycleState::Paused
            )
        })
        .collect::<Vec<_>>();
    loops.sort_by(|left, right| left.loop_meta.id.cmp(&right.loop_meta.id));

    let mut lines = vec![format!(
        "govctl project: {} ({})",
        config.project.name,
        config.project_root().display()
    )];
    if active_work.is_empty() && loops.is_empty() {
        lines.push("No active Work Item or open loop.".to_string());
        return Ok(bound_context(lines.join("\n")));
    }

    if !active_work.is_empty() {
        lines.push("Active work:".to_string());
        for work in active_work.iter().take(HOOK_ITEM_LIMIT) {
            let refs = if work.meta().refs.is_empty() {
                String::new()
            } else {
                format!(" [refs: {}]", work.meta().refs.join(", "))
            };
            lines.push(format!(
                "- {}: {}{}",
                work.meta().id,
                work.meta().title,
                refs
            ));
        }
        append_omitted_count(&mut lines, active_work.len());
    }

    if !loops.is_empty() {
        lines.push("Open loops:".to_string());
        for state in loops.iter().take(HOOK_ITEM_LIMIT) {
            lines.push(format!(
                "- {}: {} round {}, next {} [work: {}]",
                state.loop_meta.id,
                state.loop_meta.state.as_str(),
                state.loop_meta.current_round,
                state.loop_meta.next_action.as_str(),
                state.loop_meta.work.join(", ")
            ));
        }
        append_omitted_count(&mut lines, loops.len());
    }

    if let Some(state) = loops.first() {
        lines.push(format!("Next: govctl loop resume {}", state.loop_meta.id));
    } else if let Some(work) = active_work.first() {
        lines.push(format!("Next: govctl work show {}", work.meta().id));
    }
    Ok(bound_context(lines.join("\n")))
}

fn append_omitted_count(lines: &mut Vec<String>, total: usize) {
    if total > HOOK_ITEM_LIMIT {
        lines.push(format!("- ... {} more", total - HOOK_ITEM_LIMIT));
    }
}

fn recovery_context(diagnostic: &Diagnostic) -> String {
    bound_context(format!(
        "govctl governance state was detected but could not be loaded: {}. \
Repair the reported state or run `govctl check` for full diagnostics.",
        diagnostic
    ))
}

fn handle_pre_tool_use(input: &HookInput) -> DiagnosticResult<Diagnostics> {
    let Ok(Some(project_root)) = Config::governed_root_from(&input.cwd) else {
        return Ok(vec![]);
    };
    if !managed_edit_target(&project_root, input) {
        return Ok(vec![]);
    }
    emit_hook_context(AgentHookEvent::PreToolUse, DIRECT_EDIT_ADVISORY)
}

fn managed_edit_target(project_root: &Path, input: &HookInput) -> bool {
    if let Some(path) = input.tool_input.get("file_path").and_then(Value::as_str) {
        return is_lifecycle_managed_path(project_root, &input.cwd, Path::new(path));
    }

    input
        .tool_input
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| {
            command.lines().any(|line| {
                patch_path(line).is_some_and(|path| {
                    is_lifecycle_managed_path(project_root, &input.cwd, Path::new(path))
                })
            })
        })
}

fn patch_path(line: &str) -> Option<&str> {
    [
        "*** Add File: ",
        "*** Update File: ",
        "*** Delete File: ",
        "*** Move to: ",
    ]
    .iter()
    .find_map(|prefix| line.strip_prefix(prefix))
}

fn is_lifecycle_managed_path(project_root: &Path, cwd: &Path, path: &Path) -> bool {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let absolute = normalize_path(&absolute);
    let project_root = normalize_path(project_root);
    let Ok(relative) = absolute.strip_prefix(project_root) else {
        return false;
    };
    let mut components = relative.components();
    if components.next().and_then(|part| part.as_os_str().to_str()) != Some("gov") {
        return false;
    }
    match components.next().and_then(|part| part.as_os_str().to_str()) {
        Some("rfc" | "adr" | "work" | "guard" | "conformance") => true,
        Some("releases.toml") => components.next().is_none(),
        _ => false,
    }
}

fn bound_context(context: String) -> String {
    if context.chars().count() <= HOOK_CONTEXT_CHAR_LIMIT {
        return context;
    }
    let suffix = "\n... context truncated";
    let keep = HOOK_CONTEXT_CHAR_LIMIT.saturating_sub(suffix.chars().count());
    context.chars().take(keep).chain(suffix.chars()).collect()
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn emit_hook_context(event: AgentHookEvent, context: &str) -> DiagnosticResult<Diagnostics> {
    let event_name = match event {
        AgentHookEvent::SessionStart => "SessionStart",
        AgentHookEvent::PreToolUse => "PreToolUse",
    };
    let output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": event_name,
            "additionalContext": context,
        }
    });
    let output = serde_json::to_string(&output)
        .map_err(|err| hook_error(format!("Cannot serialize hook output: {err}")))?;
    println!("{output}");
    Ok(vec![])
}

fn hook_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1401AgentIntegrationFailed,
        message,
        "agent hook",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_context_has_a_hard_character_limit() {
        let context = bound_context("界".repeat(HOOK_CONTEXT_CHAR_LIMIT + 100));
        assert_eq!(context.chars().count(), HOOK_CONTEXT_CHAR_LIMIT);
    }

    #[test]
    fn invalid_projects_still_scope_managed_artifact_edits()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let gov_root = temp.path().join("gov");
        std::fs::create_dir_all(gov_root.join("work"))?;
        std::fs::write(gov_root.join("config.toml"), "invalid = [")?;

        let project_root =
            Config::governed_root_from(temp.path())?.ok_or("missing governed project root")?;
        let input = HookInput {
            cwd: temp.path().to_path_buf(),
            tool_input: serde_json::json!({
                "file_path": gov_root.join("work/recovery.toml"),
            }),
        };
        assert!(managed_edit_target(&project_root, &input));
        Ok(())
    }
}
