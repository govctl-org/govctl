//! User-scoped agent integration management.
//!
//! Implements [[RFC-0002:C-AGENT-INTEGRATION]] and [[ADR-0061]].

use agent_plugin_installer::{
    AgentPluginOperation, AgentRuntime, AgentSelector, BatchFailure, BatchOperationReport,
    BatchRuntimeOutcome, BatchSkipReason, BatchStatus, DoctorStatus, FailurePolicy, InstallRequest,
    MarketplaceSource, PluginRef, check_operation, install_many, update_from_source_many,
};
use directories::{BaseDirs, ProjectDirs};
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::ui;
use crate::write::{WriteOp, create_dir_all, write_file};

include!(concat!(env!("OUT_DIR"), "/agent_plugin_assets.rs"));
include!(concat!(env!("OUT_DIR"), "/agent_codex_templates.rs"));

const MARKETPLACE_NAME: &str = "govctl-bundled";
const PLUGIN_NAME: &str = "govctl";
const PLUGIN_SELECTOR: &str = "govctl@govctl-bundled";

struct AgentLocations {
    marketplace_root: PathBuf,
    codex_home: PathBuf,
}

pub fn manage(
    operation: AgentPluginOperation,
    selector: AgentSelector,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    match operation {
        AgentPluginOperation::Doctor => doctor(selector),
        AgentPluginOperation::Install | AgentPluginOperation::Update => {
            install_or_update(operation, selector, op)
        }
        _ => Err(agent_error("Unsupported agent integration operation")),
    }
}

fn doctor(selector: AgentSelector) -> DiagnosticResult<Diagnostics> {
    let mut failures = Vec::new();
    for runtime in selector.runtimes() {
        // Update is the broadest readiness probe: Claude checks both native
        // install and update support, while Codex validates its install path.
        match check_runtime(*runtime, AgentPluginOperation::Update) {
            Ok(()) => ui::sub_info(format!("{} agent operations are ready", runtime.id())),
            Err(message) => failures.push(format!("{}: {message}", runtime.id())),
        }
    }
    if failures.is_empty() {
        ui::success("Agent plugin operations are ready");
        Ok(vec![])
    } else {
        Err(agent_error(failures.join("; ")))
    }
}

fn install_or_update(
    operation: AgentPluginOperation,
    selector: AgentSelector,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let locations = resolve_locations()?;
    validate_bundled_versions()?;

    if op.is_preview() {
        preflight(selector, operation)?;
        materialize_plugin(&locations.marketplace_root, op)?;
        if selects_codex(selector) {
            project_codex_roles(&locations.codex_home, operation, op).map_err(|diagnostic| {
                agent_error(format!(
                    "Codex reviewer-role projection failed: {diagnostic}"
                ))
            })?;
        }
        ui::info(format!(
            "Would run native {} for {}",
            operation_name(operation),
            selector
        ));
        return Ok(vec![]);
    }

    materialize_plugin(&locations.marketplace_root, op)?;
    let result = match operation {
        AgentPluginOperation::Install => install_many(
            selector,
            |runtime| install_request(runtime, &locations.marketplace_root),
            FailurePolicy::StopOnFailure,
        ),
        AgentPluginOperation::Update => update_from_source_many(
            selector,
            |runtime| install_request(runtime, &locations.marketplace_root),
            FailurePolicy::StopOnFailure,
        ),
        _ => return Err(agent_error("Unsupported mutating agent operation")),
    };

    let report = match result {
        Ok(report) => report,
        Err(error) => return Err(report_error(error.into_report())),
    };
    report_success(&report);

    if selects_codex(selector) {
        project_codex_roles(&locations.codex_home, operation, op).map_err(|diagnostic| {
            agent_error(format!(
                "Codex reviewer-role projection failed after native plugin installation: \
                 {diagnostic}"
            ))
        })?;
    }
    ui::hint("Start a new agent session to load the refreshed integration.");
    Ok(vec![])
}

fn preflight(selector: AgentSelector, operation: AgentPluginOperation) -> DiagnosticResult<()> {
    let mut failures = Vec::new();
    for runtime in selector.runtimes() {
        if let Err(message) = check_runtime(*runtime, operation) {
            failures.push(format!("{}: {message}", runtime.id()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(agent_error(failures.join("; ")))
    }
}

fn check_runtime(runtime: AgentRuntime, operation: AgentPluginOperation) -> Result<(), String> {
    let probes: &[AgentPluginOperation] = match (runtime, operation) {
        (_, AgentPluginOperation::Install)
        | (AgentRuntime::Codex, AgentPluginOperation::Update) => &[AgentPluginOperation::Install],
        (AgentRuntime::Claude, AgentPluginOperation::Update) => {
            &[AgentPluginOperation::Install, AgentPluginOperation::Update]
        }
        _ => return Err("unsupported agent integration operation".to_string()),
    };
    for probe in probes {
        let outcome = check_operation(runtime, *probe);
        if outcome.status != DoctorStatus::Ready {
            return Err(outcome
                .message
                .unwrap_or_else(|| "runtime is not ready".to_string()));
        }
    }
    Ok(())
}

fn resolve_locations() -> DiagnosticResult<AgentLocations> {
    let project_dirs = ProjectDirs::from("org", "govctl", "govctl")
        .ok_or_else(|| agent_error("Cannot resolve a persistent user data directory"))?;
    let base_dirs = BaseDirs::new()
        .ok_or_else(|| agent_error("Cannot resolve the current user's home directory"))?;
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| base_dirs.home_dir().join(".codex"));
    Ok(AgentLocations {
        marketplace_root: project_dirs.data_local_dir().join("agent-plugin"),
        codex_home,
    })
}

fn install_request(runtime: AgentRuntime, marketplace_root: &Path) -> InstallRequest<'static> {
    let request = InstallRequest::new(
        MarketplaceSource::local(marketplace_root),
        PluginRef {
            selector: PLUGIN_SELECTOR,
            name: PLUGIN_NAME,
        },
    );
    if runtime == AgentRuntime::Claude {
        request
            .with_marketplace_scope("user")
            .with_plugin_scope("user")
    } else {
        request
    }
}

fn materialize_plugin(marketplace_root: &Path, op: WriteOp) -> DiagnosticResult<()> {
    let version = env!("CARGO_PKG_VERSION");
    let plugin_root = marketplace_root
        .join("versions")
        .join(version)
        .join(".claude");
    let marketplace_path = marketplace_root.join(".claude-plugin/marketplace.json");
    write_support_file(
        &marketplace_path,
        &render_marketplace_manifest(version)?,
        op,
    )?;
    for (relative, content) in PLUGIN_ASSETS {
        write_support_file(&plugin_root.join(relative), content, op)?;
    }
    Ok(())
}

fn write_support_file(path: &Path, content: &str, op: WriteOp) -> DiagnosticResult<()> {
    if op.is_preview() {
        ui::info(format!("Would write: {}", path.display()));
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        create_dir_all(parent, op, Some(parent))?;
    }
    write_file(path, content, op, Some(path))
}

fn render_marketplace_manifest(version: &str) -> DiagnosticResult<String> {
    let mut manifest: Value = serde_json::from_str(PLUGIN_MARKETPLACE_TEMPLATE).map_err(|err| {
        agent_error(format!(
            "Bundled marketplace manifest is invalid JSON: {err}"
        ))
    })?;
    manifest["name"] = Value::String(MARKETPLACE_NAME.to_string());
    let plugin = manifest
        .get_mut("plugins")
        .and_then(Value::as_array_mut)
        .and_then(|plugins| plugins.first_mut())
        .ok_or_else(|| agent_error("Bundled marketplace has no plugin entry"))?;
    plugin["version"] = Value::String(version.to_string());
    plugin["source"] = Value::String(format!("./versions/{version}/.claude"));
    let mut output = serde_json::to_string_pretty(&manifest)
        .map_err(|err| agent_error(format!("Cannot serialize bundled marketplace: {err}")))?;
    output.push('\n');
    Ok(output)
}

fn validate_bundled_versions() -> DiagnosticResult<()> {
    let expected = env!("CARGO_PKG_VERSION");
    let mut checked = 0;
    for (relative, content) in PLUGIN_ASSETS {
        if !relative.ends_with("plugin.json") {
            continue;
        }
        let manifest: Value = serde_json::from_str(content).map_err(|err| {
            agent_error(format!(
                "Bundled plugin manifest {relative} is invalid: {err}"
            ))
        })?;
        let actual = manifest.get("version").and_then(Value::as_str);
        if actual != Some(expected) {
            return Err(agent_error(format!(
                "Bundled plugin manifest {relative} has version {}, expected {expected}",
                actual.unwrap_or("<missing>")
            )));
        }
        checked += 1;
    }
    if checked != 2 {
        return Err(agent_error(format!(
            "Expected two bundled plugin manifests, found {checked}"
        )));
    }
    Ok(())
}

fn project_codex_roles(
    codex_home: &Path,
    operation: AgentPluginOperation,
    op: WriteOp,
) -> DiagnosticResult<()> {
    for (relative, content) in AGENT_TEMPLATES_CODEX {
        let path = codex_home.join(relative);
        if operation == AgentPluginOperation::Install && path.exists() {
            ui::sub_info(format!("Skipped {} (already exists)", path.display()));
            continue;
        }
        write_support_file(&path, content, op)?;
    }
    Ok(())
}

fn selects_codex(selector: AgentSelector) -> bool {
    selector.runtimes().contains(&AgentRuntime::Codex)
}

fn report_success(report: &BatchOperationReport) {
    for outcome in &report.outcomes {
        ui::success(format!(
            "{} agent integration {}",
            outcome.runtime.id(),
            operation_past_tense(report.operation)
        ));
    }
}

fn report_error(report: BatchOperationReport) -> Diagnostic {
    let details = report
        .outcomes
        .iter()
        .map(format_outcome)
        .collect::<Vec<_>>()
        .join("; ");
    agent_error(format!(
        "Agent integration {} failed: {details}",
        operation_name(report.operation)
    ))
}

fn format_outcome(outcome: &BatchRuntimeOutcome) -> String {
    let commands = if outcome.commands.is_empty() {
        String::new()
    } else {
        format!("; commands: {}", outcome.commands.join(", "))
    };
    match outcome.status {
        BatchStatus::Succeeded => format!("{} succeeded{commands}", outcome.runtime.id()),
        BatchStatus::Missing | BatchStatus::Failed => {
            let failure = outcome
                .failure
                .as_ref()
                .map(format_failure)
                .unwrap_or_else(|| "unknown failure".to_string());
            format!("{} failed: {failure}{commands}", outcome.runtime.id())
        }
        BatchStatus::Skipped => {
            let reason = outcome
                .skip_reason
                .map(BatchSkipReason::message)
                .unwrap_or("operation was skipped");
            format!("{} skipped: {reason}{commands}", outcome.runtime.id())
        }
        _ => format!(
            "{} returned an unknown status{commands}",
            outcome.runtime.id()
        ),
    }
}

fn format_failure(failure: &BatchFailure) -> String {
    match failure {
        BatchFailure::Validation(error) => error.to_string(),
        BatchFailure::Preflight { message } => message.clone(),
        BatchFailure::Operation(error) => error.to_string(),
        _ => "unknown failure".to_string(),
    }
}

fn operation_name(operation: AgentPluginOperation) -> &'static str {
    match operation {
        AgentPluginOperation::Doctor => "doctor",
        AgentPluginOperation::Install => "install",
        AgentPluginOperation::Update => "update",
        AgentPluginOperation::Uninstall => "uninstall",
    }
}

fn operation_past_tense(operation: AgentPluginOperation) -> &'static str {
    match operation {
        AgentPluginOperation::Install => "installed",
        AgentPluginOperation::Update => "updated",
        AgentPluginOperation::Doctor => "checked",
        AgentPluginOperation::Uninstall => "uninstalled",
    }
}

fn agent_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1401AgentIntegrationFailed,
        message,
        "agent integration",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketplace_points_to_versioned_bundled_plugin() -> Result<(), Box<dyn std::error::Error>> {
        let rendered = render_marketplace_manifest("1.2.3")?;
        let value: Value = serde_json::from_str(&rendered)?;
        assert_eq!(value["name"], MARKETPLACE_NAME);
        assert_eq!(value["plugins"][0]["version"], "1.2.3");
        assert_eq!(value["plugins"][0]["source"], "./versions/1.2.3/.claude");
        Ok(())
    }

    #[test]
    fn bundled_plugin_manifests_match_binary_version() -> Result<(), Diagnostic> {
        validate_bundled_versions()
    }

    #[test]
    fn codex_install_preserves_existing_role_and_adds_missing_roles()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let existing = temp.path().join("agents/rfc-reviewer.toml");
        std::fs::create_dir_all(existing.parent().ok_or("missing parent")?)?;
        std::fs::write(&existing, "custom = true\n")?;

        project_codex_roles(temp.path(), AgentPluginOperation::Install, WriteOp::Execute)?;

        assert_eq!(std::fs::read_to_string(existing)?, "custom = true\n");
        let added = std::fs::read_to_string(temp.path().join("agents/adr-reviewer.toml"))?;
        let value: toml::Value = toml::from_str(&added)?;
        assert_eq!(value["sandbox_mode"].as_str(), Some("read-only"));
        assert!(value["developer_instructions"].as_str().is_some());
        Ok(())
    }

    #[test]
    fn codex_update_replaces_existing_roles() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let existing = temp.path().join("agents/rfc-reviewer.toml");
        std::fs::create_dir_all(existing.parent().ok_or("missing parent")?)?;
        std::fs::write(&existing, "custom = true\n")?;

        project_codex_roles(temp.path(), AgentPluginOperation::Update, WriteOp::Execute)?;

        let value: toml::Value = toml::from_str(&std::fs::read_to_string(existing)?)?;
        assert_eq!(value["name"].as_str(), Some("rfc-reviewer"));
        assert_eq!(value["sandbox_mode"].as_str(), Some("read-only"));
        Ok(())
    }
}
