//! Behavioral tests for machine-readable CLI metadata.

mod common;

use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn run(dir: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()
}

fn run_success(dir: &Path, args: &[&str]) -> common::TestResult {
    let output = run(dir, args)?;
    assert!(
        output.status.success(),
        "govctl {} failed:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn describe_json(dir: &Path, context: bool) -> Result<Value, Box<dyn std::error::Error>> {
    let args = if context {
        vec!["describe", "--context"]
    } else {
        vec!["describe"]
    };
    let output = run(dir, &args)?;
    assert!(
        output.status.success(),
        "govctl describe failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn assert_context_failure(dir: &Path, expected_code: &str) -> common::TestResult {
    let output = run(dir, &["describe", "--context"])?;
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(expected_code));
    Ok(())
}

fn missing(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

fn command_named<'a>(commands: &'a [Value], name: &str) -> Result<&'a Value, std::io::Error> {
    commands
        .iter()
        .find(|command| command["name"] == name)
        .ok_or_else(|| missing(format!("missing command {name}")))
}

fn assert_lexicographic_command_order(commands: &[Value]) -> Result<(), std::io::Error> {
    let names = commands
        .iter()
        .map(|command| {
            command["name"]
                .as_str()
                .ok_or_else(|| missing("command name"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert!(names.windows(2).all(|pair| pair[0] <= pair[1]));

    for command in commands {
        assert_lexicographic_command_order(
            command["subcommands"]
                .as_array()
                .ok_or_else(|| missing("subcommands array"))?,
        )?;
    }
    Ok(())
}

#[test]
fn describe_reports_versioned_parser_derived_shape() -> common::TestResult {
    let (temp_dir, _) = common::temp_dir_with_date()?;
    let value = describe_json(temp_dir.path(), false)?;

    assert_eq!(value["schema_version"], 1);
    assert!(value["tool_version"].as_str().is_some());
    assert_eq!(
        value["workflow"]["phase_order"],
        serde_json::json!(["spec", "impl", "test", "stable"])
    );
    assert!(value["workflow"].get("typical_sequence").is_none());
    assert!(value.get("project_state").is_none());
    assert!(value.get("suggested_actions").is_none());

    let commands = value["commands"]
        .as_array()
        .ok_or_else(|| missing("command array"))?;
    let agent = command_named(commands, "agent")?;
    let agent_subcommands = agent["subcommands"]
        .as_array()
        .ok_or_else(|| missing("agent subcommands"))?;
    assert_eq!(
        agent_subcommands
            .iter()
            .map(|command| command["name"].as_str())
            .collect::<Vec<_>>(),
        [Some("doctor"), Some("install"), Some("update")]
    );
    let rfc = command_named(commands, "rfc")?;
    let edit = command_named(
        rfc["subcommands"]
            .as_array()
            .ok_or_else(|| missing("RFC subcommands"))?,
        "edit",
    )?;
    assert_eq!(edit["usage"], "govctl rfc edit [OPTIONS] <ID> <PATH>");
    assert!(
        edit["summary"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );
    assert!(edit.get("example").is_none());
    assert!(edit.get("prerequisites").is_none());
    assert!(edit.get("when_to_use").is_none());
    assert!(commands.iter().all(|command| command["name"] != "help"));
    assert_lexicographic_command_order(commands)?;

    let unsupported_output = run(temp_dir.path(), &["describe", "--output", "json"])?;
    assert!(!unsupported_output.status.success());
    Ok(())
}

#[test]
fn describe_context_reports_empty_state_with_explicit_zero_counts() -> common::TestResult {
    let (temp_dir, _) = common::init_project_with_date()?;
    let value = describe_json(temp_dir.path(), true)?;
    let state = &value["project_state"];

    assert_eq!(state["counts"]["rfcs"]["total"], 0);
    assert_eq!(state["counts"]["adrs"]["accepted"], 0);
    assert_eq!(state["counts"]["work_items"]["done"], 0);
    assert_eq!(state["counts"]["loops"]["failed"], 0);
    assert_eq!(state["rfcs"], serde_json::json!([]));
    assert_eq!(state["adrs"], serde_json::json!([]));
    assert_eq!(state["work_items"], serde_json::json!([]));
    assert_eq!(state["loops"], serde_json::json!([]));
    assert_eq!(value["suggested_actions"], serde_json::json!([]));
    Ok(())
}

#[test]
fn describe_context_enumerates_only_actionable_artifacts() -> common::TestResult {
    let (temp_dir, date) = common::init_project_with_date()?;
    let dir = temp_dir.path();

    for args in [
        vec!["rfc", "new", "Stable RFC"],
        vec!["rfc", "finalize", "RFC-0001", "normative"],
        vec!["rfc", "advance", "RFC-0001", "impl"],
        vec!["rfc", "advance", "RFC-0001", "test"],
        vec!["rfc", "advance", "RFC-0001", "stable"],
        vec!["rfc", "new", "Draft RFC"],
        vec!["adr", "new", "Open decision"],
        vec!["work", "new", "Queued work"],
        vec!["work", "new", "Active work", "--active"],
        vec!["work", "new", "Completed work", "--active"],
    ] {
        run_success(dir, &args)?;
    }

    let done_id = common::work_id(&date, 3);
    run_success(
        dir,
        &[
            "work",
            "edit",
            &done_id,
            "acceptance_criteria",
            "--add",
            "chore: completed",
        ],
    )?;
    run_success(
        dir,
        &[
            "work",
            "edit",
            &done_id,
            "acceptance_criteria[0]",
            "--tick",
            "done",
        ],
    )?;
    run_success(dir, &["work", "move", &done_id, "done"])?;

    let value = describe_json(dir, true)?;
    let state = &value["project_state"];
    assert_eq!(state["counts"]["rfcs"]["total"], 2);
    assert_eq!(state["counts"]["rfcs"]["stable"], 1);
    assert_eq!(state["counts"]["work_items"]["done"], 1);

    let rfcs = state["rfcs"]
        .as_array()
        .ok_or_else(|| missing("actionable RFCs"))?;
    assert_eq!(rfcs.len(), 1);
    assert_eq!(rfcs[0]["id"], "RFC-0002");
    let work_items = state["work_items"]
        .as_array()
        .ok_or_else(|| missing("actionable Work Items"))?;
    assert_eq!(work_items.len(), 2);
    assert!(
        work_items
            .iter()
            .all(|work_item| work_item["id"] != done_id)
    );

    let actions = value["suggested_actions"]
        .as_array()
        .ok_or_else(|| missing("suggested actions"))?;
    assert!(
        actions
            .iter()
            .all(|action| !action.as_str().unwrap_or_default().contains(&done_id))
    );
    assert!(
        actions
            .iter()
            .all(|action| { !action.as_str().unwrap_or_default().contains("RFC-0001") })
    );
    Ok(())
}

#[test]
fn describe_context_includes_non_terminal_loops() -> common::TestResult {
    let (temp_dir, date) = common::init_project_with_date()?;
    let dir = temp_dir.path();
    let work_id = common::first_work_id(&date);
    run_success(dir, &["work", "new", "Loop work", "--active"])?;
    run_success(dir, &["loop", "start", &work_id])?;
    let loop_root = dir.join(".govctl/loops");
    let loop_dir = std::fs::read_dir(&loop_root)?
        .next()
        .ok_or_else(|| missing("generated loop directory"))??
        .path();
    let state_path = loop_dir.join("state.toml");
    let state = std::fs::read_to_string(&state_path)?;
    std::fs::write(&state_path, state.replace("next_action = \"start\"\n", ""))?;

    let value = describe_json(dir, true)?;
    let state = &value["project_state"];
    assert_eq!(state["counts"]["loops"]["total"], 1);
    assert_eq!(state["counts"]["loops"]["pending"], 1);
    let loops = state["loops"]
        .as_array()
        .ok_or_else(|| missing("open loops"))?;
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0]["state"], "pending");
    assert_eq!(loops[0]["next_action"], "start");
    assert_eq!(loops[0]["work"], serde_json::json!([work_id]));
    assert!(
        value["suggested_actions"]
            .as_array()
            .is_some_and(|actions| {
                actions.iter().any(|action| {
                    action
                        .as_str()
                        .is_some_and(|text| text.starts_with("govctl loop resume LOOP-"))
                })
            })
    );
    Ok(())
}

#[test]
fn describe_context_reports_load_failures() -> common::TestResult {
    let (temp_dir, _) = common::init_project_with_date()?;
    let loop_dir = temp_dir.path().join(".govctl/loops/LOOP-2026-07-26-001");
    std::fs::create_dir_all(&loop_dir)?;
    std::fs::write(loop_dir.join("state.toml"), "not valid loop state")?;

    assert_context_failure(temp_dir.path(), "E1201")
}

#[test]
fn describe_context_rejects_malformed_adrs() -> common::TestResult {
    let (temp_dir, _) = common::init_project_with_date()?;
    std::fs::write(
        temp_dir.path().join("gov/adr/ADR-0001-invalid.toml"),
        "not valid ADR TOML",
    )?;

    assert_context_failure(temp_dir.path(), "E0301")
}

#[test]
fn describe_context_rejects_malformed_work_items() -> common::TestResult {
    let (temp_dir, _) = common::init_project_with_date()?;
    std::fs::write(
        temp_dir
            .path()
            .join("gov/work/2026-07-26-invalid-work.toml"),
        "not valid Work Item TOML",
    )?;

    assert_context_failure(temp_dir.path(), "E0401")
}
