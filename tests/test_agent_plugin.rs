//! Behavioral tests for [[RFC-0002:C-AGENT-INTEGRATION]].

mod common;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run_agent(
    cwd: &Path,
    home: &Path,
    path: OsString,
    log: &Path,
    args: &[&str],
) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env("CODEX_HOME", home.join(".codex"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("PATH", path)
        .env("AGENT_PLUGIN_LOG", log)
        .env("NO_COLOR", "1")
        .output()
}

fn run_hook(
    cwd: &Path,
    args: &[&str],
    input: serde_json::Value,
) -> Result<Output, Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing hook stdin")?
        .write_all(serde_json::to_string(&input)?.as_bytes())?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

fn hook_context(output: &Output) -> Result<String, Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "missing hook additionalContext".into())
}

#[test]
fn agent_doctor_is_project_independent_and_fails_for_missing_runtime() -> common::TestResult {
    let temp = tempfile::tempdir()?;
    let output = run_agent(
        temp.path(),
        &temp.path().join("home"),
        OsString::new(),
        &temp.path().join("agent.log"),
        &["agent", "doctor", "codex"],
    )?;

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E1401"));
    assert!(!temp.path().join("gov").exists());
    Ok(())
}

#[cfg(unix)]
fn fake_runtime_path(root: &Path) -> Result<OsString, Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let bin = root.join("bin");
    fs::create_dir_all(&bin)?;
    let script = r#"#!/bin/sh
printf '%s %s\n' "${0##*/}" "$*" >> "$AGENT_PLUGIN_LOG"
case " $* " in
  *" --help "*) ;;
  *)
    if [ -f "${AGENT_PLUGIN_LOG}.fail-${0##*/}" ]; then
      printf 'simulated %s failure\n' "${0##*/}" >&2
      exit 9
    fi
    ;;
esac
exit 0
"#;
    for runtime in ["codex", "claude"] {
        let path = bin.join(runtime);
        fs::write(&path, script)?;
        let mut permissions = fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }
    Ok(bin.into_os_string())
}

#[cfg(unix)]
fn marketplace_root_from_log(log: &str) -> Result<PathBuf, std::io::Error> {
    log.lines()
        .find_map(|line| {
            line.strip_prefix("codex plugin marketplace add ")
                .filter(|rest| !rest.ends_with("--help"))
                .map(PathBuf::from)
        })
        .ok_or_else(|| std::io::Error::other("missing Codex marketplace add operation"))
}

#[cfg(unix)]
#[test]
fn agent_doctor_preflights_all_selected_runtimes() -> common::TestResult {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    let log_path = temp.path().join("agent.log");
    let output = run_agent(
        temp.path(),
        &home,
        fake_runtime_path(temp.path())?,
        &log_path,
        &["agent", "doctor", "all"],
    )?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = fs::read_to_string(log_path)?;
    assert!(log.lines().any(|line| line.starts_with("codex ")));
    assert!(log.lines().any(|line| line.starts_with("claude ")));
    assert!(log.lines().all(|line| line.ends_with("--help")));
    assert!(!home.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn agent_dry_run_preflights_without_persistent_writes() -> common::TestResult {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    let log_path = temp.path().join("agent.log");
    let output = run_agent(
        temp.path(),
        &home,
        fake_runtime_path(temp.path())?,
        &log_path,
        &["--dry-run", "agent", "install", "all"],
    )?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = fs::read_to_string(log_path)?;
    assert!(!log.is_empty(), "dry run should preflight both runtimes");
    assert!(log.lines().all(|line| line.ends_with("--help")));
    assert!(!home.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn agent_batch_failure_reports_failed_and_skipped_runtimes() -> common::TestResult {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    let log_path = temp.path().join("agent.log");
    fs::write(
        format!("{}.fail-codex", log_path.display()),
        "fail mutation",
    )?;
    let output = run_agent(
        temp.path(),
        &home,
        fake_runtime_path(temp.path())?,
        &log_path,
        &["agent", "install", "all"],
    )?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E1401"));
    assert!(stderr.contains("codex failed"));
    assert!(stderr.contains("claude skipped"));
    assert!(!home.join(".codex/agents").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn agent_install_and_update_use_native_plugins_and_codex_roles() -> common::TestResult {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    let codex_agents = home.join(".codex/agents");
    fs::create_dir_all(&codex_agents)?;
    let existing_role = codex_agents.join("rfc-reviewer.toml");
    fs::write(&existing_role, "custom = true\n")?;
    let log_path = temp.path().join("agent.log");
    let fake_path = fake_runtime_path(temp.path())?;

    let install = run_agent(
        temp.path(),
        &home,
        fake_path.clone(),
        &log_path,
        &["agent", "install", "all"],
    )?;
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    assert_eq!(fs::read_to_string(&existing_role)?, "custom = true\n");
    let added: toml::Value =
        toml::from_str(&fs::read_to_string(codex_agents.join("adr-reviewer.toml"))?)?;
    assert_eq!(added["sandbox_mode"].as_str(), Some("read-only"));

    let install_log = fs::read_to_string(&log_path)?;
    let lines = install_log.lines().collect::<Vec<_>>();
    let install_line_count = lines.len();
    let first_mutation = lines
        .iter()
        .position(|line| !line.ends_with("--help"))
        .ok_or("missing native mutation")?;
    assert!(
        lines[..first_mutation]
            .iter()
            .any(|line| line.starts_with("codex "))
    );
    assert!(
        lines[..first_mutation]
            .iter()
            .any(|line| line.starts_with("claude "))
    );
    assert!(
        lines[first_mutation..]
            .iter()
            .any(|line| line == &"codex plugin add govctl@govctl-bundled")
    );
    assert!(
        lines[first_mutation..]
            .iter()
            .any(|line| line == &"claude plugin install --scope user govctl@govctl-bundled")
    );

    let marketplace_root = marketplace_root_from_log(&install_log)?;
    let marketplace: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        marketplace_root.join(".claude-plugin/marketplace.json"),
    )?)?;
    assert_eq!(marketplace["name"], "govctl-bundled");
    let source = marketplace["plugins"][0]["source"]
        .as_str()
        .ok_or("missing plugin source")?;
    let plugin_root = marketplace_root.join(source);
    assert!(plugin_root.join(".claude-plugin/plugin.json").exists());
    let codex_manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        plugin_root.join(".codex-plugin/plugin.json"),
    )?)?;
    assert_eq!(codex_manifest["skills"], "./skills/");
    assert_eq!(codex_manifest["hooks"], "./hooks/codex.json");
    let claude_manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        plugin_root.join(".claude-plugin/plugin.json"),
    )?)?;
    assert_eq!(claude_manifest["hooks"], "./hooks/claude.json");
    assert!(plugin_root.join("skills/gov/SKILL.md").exists());
    let codex_hooks: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(plugin_root.join("hooks/codex.json"))?)?;
    let claude_hooks: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(plugin_root.join("hooks/claude.json"))?)?;
    assert!(codex_hooks["hooks"].get("Stop").is_none());
    assert!(claude_hooks["hooks"].get("Stop").is_none());
    assert_eq!(
        codex_hooks["hooks"]["SessionStart"][0]["hooks"][0]["command"],
        "govctl agent hook session-start"
    );
    assert_eq!(
        claude_hooks["hooks"]["PreToolUse"][0]["matcher"],
        "Edit|Write"
    );
    assert!(
        codex_hooks["hooks"]["SessionStart"][0]["hooks"][0]
            .get("additionalContextLimit")
            .is_some()
    );
    assert!(
        claude_hooks["hooks"]["SessionStart"][0]["hooks"][0]
            .get("additionalContextLimit")
            .is_none()
    );

    let update = run_agent(
        temp.path(),
        &home,
        fake_path,
        &log_path,
        &["agent", "update", "all"],
    )?;
    assert!(
        update.status.success(),
        "{}",
        String::from_utf8_lossy(&update.stderr)
    );
    let updated: toml::Value = toml::from_str(&fs::read_to_string(existing_role)?)?;
    assert_eq!(updated["name"].as_str(), Some("rfc-reviewer"));
    assert_eq!(updated["sandbox_mode"].as_str(), Some("read-only"));
    let update_log = fs::read_to_string(log_path)?;
    let update_lines = update_log
        .lines()
        .skip(install_line_count)
        .collect::<Vec<_>>();
    let first_update_mutation = update_lines
        .iter()
        .position(|line| !line.ends_with("--help"))
        .ok_or("missing native update mutation")?;
    assert!(
        update_lines[..first_update_mutation]
            .iter()
            .any(|line| line.starts_with("codex "))
    );
    assert!(
        update_lines[..first_update_mutation]
            .iter()
            .any(|line| line.starts_with("claude "))
    );
    let update_mutations = &update_lines[first_update_mutation..];
    assert!(
        update_mutations
            .iter()
            .any(|line| line.starts_with("codex plugin marketplace add "))
    );
    assert!(
        update_mutations
            .iter()
            .any(|line| line == &"claude plugin update --scope user govctl@govctl-bundled")
    );
    assert!(
        update_mutations
            .iter()
            .all(|line| !line.contains("marketplace upgrade"))
    );
    Ok(())
}

#[test]
fn session_start_is_silent_outside_governed_projects() -> common::TestResult {
    let temp = tempfile::tempdir()?;
    let output = run_hook(
        temp.path(),
        &["agent", "hook", "session-start"],
        serde_json::json!({
            "cwd": temp.path(),
            "hook_event_name": "SessionStart"
        }),
    )?;

    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn session_start_discovers_parent_project_and_reports_active_work() -> common::TestResult {
    let temp = common::init_project()?;
    let created = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "new", "Hook context work", "--active"])
        .current_dir(temp.path())
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .env("NO_COLOR", "1")
        .output()?;
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let work_id = common::first_work_id(&common::today());
    let loop_started = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["loop", "start", &work_id])
        .current_dir(temp.path())
        .env("NO_COLOR", "1")
        .output()?;
    assert!(
        loop_started.status.success(),
        "{}",
        String::from_utf8_lossy(&loop_started.stderr)
    );
    let nested = temp.path().join("src/nested");
    fs::create_dir_all(&nested)?;

    let output = run_hook(
        temp.path(),
        &["agent", "hook", "session-start"],
        serde_json::json!({
            "cwd": nested,
            "hook_event_name": "SessionStart"
        }),
    )?;
    let context = hook_context(&output)?;
    assert!(context.contains("govctl project:"));
    assert!(context.contains("Active work:"));
    assert!(context.contains("Hook context work"));
    assert!(context.contains("Open loops:"));
    assert!(context.contains("pending round 0, next start"), "{context}");
    assert!(!context.contains("project_state"));
    Ok(())
}

#[test]
fn session_start_preserves_active_work_alongside_load_warnings() -> common::TestResult {
    let temp = common::init_project()?;
    let created = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["work", "new", "Visible despite warning", "--active"])
        .current_dir(temp.path())
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .env("NO_COLOR", "1")
        .output()?;
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    fs::write(
        temp.path().join("gov/work/malformed.toml"),
        "not valid toml = [",
    )?;

    let output = run_hook(
        temp.path(),
        &["agent", "hook", "session-start"],
        serde_json::json!({
            "cwd": temp.path(),
            "hook_event_name": "SessionStart"
        }),
    )?;
    let context = hook_context(&output)?;
    assert!(context.contains("Visible despite warning"));
    assert!(context.contains("Work Item load warnings:"));
    assert!(context.contains("E0401"));
    assert!(!context.contains("could not be loaded"));
    Ok(())
}

#[test]
fn session_start_reports_invalid_governance_state_without_failing() -> common::TestResult {
    let temp = common::init_project()?;
    fs::write(temp.path().join("gov/config.toml"), "not valid toml = [")?;

    let output = run_hook(
        temp.path(),
        &["agent", "hook", "session-start"],
        serde_json::json!({
            "cwd": temp.path(),
            "hook_event_name": "SessionStart"
        }),
    )?;
    let context = hook_context(&output)?;
    assert!(context.contains("could not be loaded"));
    assert!(context.contains("E0501"));

    let edit = run_hook(
        temp.path(),
        &["agent", "hook", "pre-tool-use"],
        serde_json::json!({
            "cwd": temp.path(),
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": temp.path().join("gov/work/recovery.toml")
            }
        }),
    )?;
    assert!(hook_context(&edit)?.contains("Direct editing is permitted"));
    Ok(())
}

#[test]
fn pre_tool_use_advises_without_blocking_managed_edits() -> common::TestResult {
    let temp = common::init_project()?;
    let claude = run_hook(
        temp.path(),
        &["agent", "hook", "pre-tool-use"],
        serde_json::json!({
            "cwd": temp.path(),
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": temp.path().join("gov/work/example.toml")
            }
        }),
    )?;
    let claude_value: serde_json::Value = serde_json::from_slice(&claude.stdout)?;
    assert!(
        claude_value["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .is_some_and(|context| context.contains("Direct editing is permitted"))
    );
    assert!(
        claude_value["hookSpecificOutput"]
            .get("permissionDecision")
            .is_none()
    );

    let nested = temp.path().join("src/nested");
    fs::create_dir_all(&nested)?;
    let codex = run_hook(
        &nested,
        &["agent", "hook", "pre-tool-use"],
        serde_json::json!({
            "cwd": nested,
            "hook_event_name": "PreToolUse",
            "tool_name": "apply_patch",
            "tool_input": {
                "command": "*** Begin Patch\n*** Update File: ../../gov/rfc/RFC-0001/rfc.toml\n*** End Patch\n"
            }
        }),
    )?;
    assert!(hook_context(&codex)?.contains("govctl check"));

    let source_edit = run_hook(
        temp.path(),
        &["agent", "hook", "pre-tool-use"],
        serde_json::json!({
            "cwd": temp.path(),
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": temp.path().join("src/main.rs")
            }
        }),
    )?;
    assert!(source_edit.stdout.is_empty());

    let config_edit = run_hook(
        temp.path(),
        &["agent", "hook", "pre-tool-use"],
        serde_json::json!({
            "cwd": temp.path(),
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": temp.path().join("gov/config.toml")
            }
        }),
    )?;
    assert!(config_edit.stdout.is_empty());
    Ok(())
}
