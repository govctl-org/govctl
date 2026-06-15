use super::*;
use crate::config::{Config, PathsConfig};
use crate::diagnostic::Diagnostic;
use crate::write::WriteOp;
use std::collections::BTreeMap;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn test_config(root: &std::path::Path) -> Config {
    Config {
        gov_root: root.join("gov"),
        paths: PathsConfig {
            docs_output: root.join("docs"),
            agent_dir: root.join(".claude"),
        },
        ..Default::default()
    }
}

fn deps(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    entries
        .iter()
        .map(|(id, deps)| {
            (
                (*id).to_string(),
                deps.iter().map(|dep| (*dep).to_string()).collect(),
            )
        })
        .collect()
}

fn assert_err_contains<T>(
    result: Result<T, Diagnostic>,
    needle: &str,
    context: &str,
) -> TestResult {
    let Err(err) = result else {
        return Err(format!("{context}: expected error containing '{needle}'").into());
    };
    if !err.to_string().contains(needle) {
        return Err(format!("error should contain '{needle}', got: {err}").into());
    }
    Ok(())
}

#[test]
fn test_loop_state_round_trips_state_toml() -> TestResult {
    let temp_dir = tempfile::TempDir::new()?;
    let config = test_config(temp_dir.path());
    let root = "WI-2026-05-31-001";
    let dependency = "WI-2026-05-31-002";

    let state = LoopState::new(
        "LOOP-2026-05-31-001",
        vec![root.to_string()],
        vec![root.to_string(), dependency.to_string()],
        deps(&[(root, &[dependency]), (dependency, &[])]),
    )?;

    write_loop_state_with_op(&config, &state, WriteOp::Execute)?;

    let state_path = temp_dir
        .path()
        .join(".govctl/loops/LOOP-2026-05-31-001/state.toml");
    assert!(state_path.exists(), "state path: {}", state_path.display());
    let state_toml = std::fs::read_to_string(&state_path)?;
    assert!(state_toml.contains("work = ["));
    assert!(state_toml.contains("resolved = ["));
    assert!(!state_toml.contains("root_work_items"));
    assert!(!state_toml.contains("work_items"));
    assert!(
        !temp_dir
            .path()
            .join("gov/.govctl/loops/LOOP-2026-05-31-001/state.toml")
            .exists(),
        "loop state must be outside governed artifacts"
    );

    let loaded = load_loop_state(&config, "LOOP-2026-05-31-001")?;
    assert_eq!(loaded, state);
    assert_eq!(loaded.loop_meta.state, LoopLifecycleState::Pending);
    assert_eq!(loaded.items[root].status, LoopWorkItemStatus::Pending);
    assert_eq!(loaded.items[root].round_count, 0);
    Ok(())
}

#[test]
fn test_loop_state_rejects_legacy_state_keys() -> TestResult {
    let temp_dir = tempfile::TempDir::new()?;
    let config = test_config(temp_dir.path());
    let loop_id = "LOOP-2026-05-31-006";
    let work_id = "WI-2026-05-31-001";
    let state_dir = temp_dir.path().join(format!(".govctl/loops/{loop_id}"));
    std::fs::create_dir_all(&state_dir)?;
    std::fs::write(
        state_dir.join("state.toml"),
        format!(
            r#"[loop]
id = "{loop_id}"
state = "pending"
work = ["{work_id}"]
resolved = ["{work_id}"]
root_work_items = ["{work_id}"]

[dependencies]
"{work_id}" = []

[items."{work_id}"]
status = "pending"
round_count = 0
"#
        ),
    )?;

    assert_err_contains(
        load_loop_state(&config, loop_id),
        "unknown field `root_work_items`",
        "legacy explicit work key must be rejected",
    )?;
    Ok(())
}

#[test]
fn test_loop_round_load_tolerates_legacy_max_rounds() -> TestResult {
    let temp_dir = tempfile::TempDir::new()?;
    let config = test_config(temp_dir.path());
    let loop_id = "LOOP-2026-05-31-007";
    let work_id = "WI-2026-05-31-001";
    let round_dir = temp_dir
        .path()
        .join(format!(".govctl/loops/{loop_id}/rounds"));
    std::fs::create_dir_all(&round_dir)?;
    std::fs::write(
        round_dir.join("round-001.toml"),
        format!(
            r#"[round]
loop_id = "{loop_id}"
round_number = 1
max_rounds = 1
status = "open"
work = ["{work_id}"]

[summary]
actions = []
changed_paths = []
verification = []
blockers = []
note_candidates = []
"#
        ),
    )?;

    let record = load_loop_round_record(&config, loop_id, 1)?;
    assert_eq!(record.round_meta.loop_id, loop_id);
    assert_eq!(record.round_meta.round_number, 1);
    assert_eq!(record.round_meta.work, vec![work_id.to_string()]);

    write_loop_round_record(&config, &record, WriteOp::Execute)?;
    let rewritten = std::fs::read_to_string(round_dir.join("round-001.toml"))?;
    assert!(!rewritten.contains("max_rounds"), "{rewritten}");
    Ok(())
}

#[test]
fn test_loop_state_updates_lifecycle_item_status_and_round_count() -> TestResult {
    let temp_dir = tempfile::TempDir::new()?;
    let config = test_config(temp_dir.path());
    let work_id = "WI-2026-05-31-001";
    let mut state = LoopState::new(
        "LOOP-2026-05-31-002",
        vec![work_id.to_string()],
        vec![work_id.to_string()],
        deps(&[(work_id, &[])]),
    )?;

    state.transition_to(LoopLifecycleState::Active)?;
    state.set_item_status(work_id, LoopWorkItemStatus::Active)?;
    assert_eq!(state.increment_round_count(work_id)?, 1);
    write_loop_state_with_op(&config, &state, WriteOp::Execute)?;

    let loaded = load_loop_state(&config, "LOOP-2026-05-31-002")?;
    assert_eq!(loaded.loop_meta.state, LoopLifecycleState::Active);
    assert_eq!(loaded.items[work_id].status, LoopWorkItemStatus::Active);
    assert_eq!(loaded.items[work_id].round_count, 1);
    Ok(())
}

#[test]
fn test_loop_state_rejects_invalid_lifecycle_transition() -> TestResult {
    let work_id = "WI-2026-05-31-001";
    let mut state = LoopState::new(
        "LOOP-2026-05-31-003",
        vec![work_id.to_string()],
        vec![work_id.to_string()],
        deps(&[(work_id, &[])]),
    )?;

    let err = state.transition_to(LoopLifecycleState::Completed);
    assert_err_contains(
        err,
        "Invalid loop transition",
        "pending -> completed must be rejected",
    )?;

    state.transition_to(LoopLifecycleState::Active)?;
    state.transition_to(LoopLifecycleState::Completed)?;
    let terminal_err = state.transition_to(LoopLifecycleState::Completed);
    assert_err_contains(
        terminal_err,
        "Invalid loop transition",
        "completed -> completed must be rejected",
    )?;
    Ok(())
}

#[test]
fn test_loop_state_rejects_invalid_ids_and_contract_violations() -> TestResult {
    let work_id = "WI-2026-05-31-001";

    validate_loop_id("LOOP-2026-05-31-001")?;

    assert_err_contains(
        validate_loop_id("loop-plain-text"),
        "LOOP-YYYY-MM-DD-NNN",
        "plain-text loop IDs must be rejected",
    )?;

    assert_err_contains(
        LoopState::new(
            "../bad",
            vec![work_id.to_string()],
            vec![work_id.to_string()],
            deps(&[(work_id, &[])]),
        ),
        "Invalid loop ID",
        "path traversal loop IDs must be rejected",
    )?;

    assert_err_contains(
        LoopState::new(
            "LOOP-2026-05-31-004",
            vec![work_id.to_string()],
            vec![work_id.to_string()],
            BTreeMap::new(),
        ),
        "missing dependency entry",
        "each work item must have a dependency entry",
    )?;

    assert_err_contains(
        LoopState::new(
            "LOOP-2026-05-31-005",
            vec![work_id.to_string()],
            vec![work_id.to_string(), work_id.to_string()],
            deps(&[(work_id, &[])]),
        ),
        "duplicate",
        "duplicate work item IDs must be rejected",
    )?;

    Ok(())
}
