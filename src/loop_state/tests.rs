use super::*;
use crate::config::{Config, PathsConfig};
use std::collections::BTreeMap;

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
    result: anyhow::Result<T>,
    needle: &str,
    context: &str,
) -> anyhow::Result<()> {
    let Err(err) = result else {
        anyhow::bail!("{context}: expected error containing '{needle}'");
    };
    if !err.to_string().contains(needle) {
        anyhow::bail!("error should contain '{needle}', got: {err}");
    }
    Ok(())
}

#[test]
fn test_loop_state_round_trips_state_toml() -> anyhow::Result<()> {
    let temp_dir = tempfile::TempDir::new()?;
    let config = test_config(temp_dir.path());
    let root = "WI-2026-05-31-001";
    let dependency = "WI-2026-05-31-002";

    let state = LoopState::new(
        "loop-1",
        vec![root.to_string()],
        vec![root.to_string(), dependency.to_string()],
        deps(&[(root, &[dependency]), (dependency, &[])]),
    )?;

    write_loop_state_with_op(&config, &state, WriteOp::Execute)?;

    let state_path = temp_dir.path().join(".govctl/loops/loop-1/state.toml");
    assert!(state_path.exists(), "state path: {}", state_path.display());
    assert!(
        !temp_dir
            .path()
            .join("gov/.govctl/loops/loop-1/state.toml")
            .exists(),
        "loop state must be outside governed artifacts"
    );

    let loaded = load_loop_state(&config, "loop-1")?;
    assert_eq!(loaded, state);
    assert_eq!(loaded.loop_meta.state, LoopLifecycleState::Pending);
    assert_eq!(loaded.items[root].status, LoopWorkItemStatus::Pending);
    assert_eq!(loaded.items[root].round_count, 0);
    Ok(())
}

#[test]
fn test_loop_state_updates_lifecycle_item_status_and_round_count() -> anyhow::Result<()> {
    let temp_dir = tempfile::TempDir::new()?;
    let config = test_config(temp_dir.path());
    let work_id = "WI-2026-05-31-001";
    let mut state = LoopState::new(
        "loop-2",
        vec![work_id.to_string()],
        vec![work_id.to_string()],
        deps(&[(work_id, &[])]),
    )?;

    state.transition_to(LoopLifecycleState::Active)?;
    state.set_item_status(work_id, LoopWorkItemStatus::Active)?;
    assert_eq!(state.increment_round_count(work_id)?, 1);
    write_loop_state_with_op(&config, &state, WriteOp::Execute)?;

    let loaded = load_loop_state(&config, "loop-2")?;
    assert_eq!(loaded.loop_meta.state, LoopLifecycleState::Active);
    assert_eq!(loaded.items[work_id].status, LoopWorkItemStatus::Active);
    assert_eq!(loaded.items[work_id].round_count, 1);
    Ok(())
}

#[test]
fn test_loop_state_rejects_invalid_lifecycle_transition() -> anyhow::Result<()> {
    let work_id = "WI-2026-05-31-001";
    let mut state = LoopState::new(
        "loop-3",
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
fn test_loop_state_rejects_invalid_ids_and_contract_violations() -> anyhow::Result<()> {
    let work_id = "WI-2026-05-31-001";

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
            "loop-4",
            vec![work_id.to_string()],
            vec![work_id.to_string()],
            BTreeMap::new(),
        ),
        "missing dependency entry",
        "each work item must have a dependency entry",
    )?;

    assert_err_contains(
        LoopState::new(
            "loop-5",
            vec![work_id.to_string()],
            vec![work_id.to_string(), work_id.to_string()],
            deps(&[(work_id, &[])]),
        ),
        "duplicate",
        "duplicate work item IDs must be rejected",
    )?;

    Ok(())
}
