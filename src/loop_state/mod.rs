use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::write::WriteOp;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

mod storage;
mod validation;

pub use validation::validate_loop_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopLifecycleState {
    Pending,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopWorkItemStatus {
    Pending,
    Active,
    Done,
    Failed,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopMeta {
    pub id: String,
    pub state: LoopLifecycleState,
    pub root_work_items: Vec<String>,
    pub work_items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopItemState {
    pub status: LoopWorkItemStatus,
    pub round_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopState {
    #[serde(rename = "loop")]
    pub loop_meta: LoopMeta,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub items: BTreeMap<String, LoopItemState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopRoundRecord {
    pub loop_id: String,
    pub work_item_id: String,
    pub round_number: u32,
    pub max_rounds: u32,
    pub item_status_before: String,
    pub item_status_after: String,
    pub work_status_before: String,
    pub work_status_after: String,
    pub action: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl LoopState {
    pub fn new(
        loop_id: impl Into<String>,
        root_work_items: Vec<String>,
        work_items: Vec<String>,
        dependencies: BTreeMap<String, Vec<String>>,
    ) -> anyhow::Result<Self> {
        let items = work_items
            .iter()
            .map(|work_id| {
                (
                    work_id.clone(),
                    LoopItemState {
                        status: LoopWorkItemStatus::Pending,
                        round_count: 0,
                    },
                )
            })
            .collect();

        let state = Self {
            loop_meta: LoopMeta {
                id: loop_id.into(),
                state: LoopLifecycleState::Pending,
                root_work_items,
                work_items,
            },
            dependencies,
            items,
        };
        state.validate(None)?;
        Ok(state)
    }

    pub fn transition_to(&mut self, next: LoopLifecycleState) -> anyhow::Result<()> {
        let current = self.loop_meta.state;
        validation::validate_loop_transition(&self.loop_meta.id, current, next)?;
        self.loop_meta.state = next;
        Ok(())
    }

    pub fn set_item_status(
        &mut self,
        work_id: &str,
        status: LoopWorkItemStatus,
    ) -> anyhow::Result<()> {
        let Some(item) = self.items.get_mut(work_id) else {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("Loop state has no item entry for work item: {work_id}"),
                self.loop_meta.id.clone(),
            )
            .into());
        };
        item.status = status;
        Ok(())
    }

    pub fn increment_round_count(&mut self, work_id: &str) -> anyhow::Result<u32> {
        let Some(item) = self.items.get_mut(work_id) else {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("Loop state has no item entry for work item: {work_id}"),
                self.loop_meta.id.clone(),
            )
            .into());
        };
        item.round_count = item.round_count.checked_add(1).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("Round count overflow for work item: {work_id}"),
                self.loop_meta.id.clone(),
            )
        })?;
        Ok(item.round_count)
    }

    pub fn validate(&self, expected_loop_id: Option<&str>) -> anyhow::Result<()> {
        validation::validate_loop_state(self, expected_loop_id)
    }
}

impl LoopRoundRecord {
    pub fn validate(&self) -> anyhow::Result<()> {
        validation::validate_loop_round_record(self)
    }
}

impl LoopLifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl LoopWorkItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
        }
    }
}

pub fn loop_state_path(config: &Config, loop_id: &str) -> anyhow::Result<PathBuf> {
    storage::loop_state_path(config, loop_id)
}

pub fn loop_state_root(config: &Config) -> PathBuf {
    storage::loop_state_root(config)
}

pub fn write_loop_state_with_op(
    config: &Config,
    state: &LoopState,
    op: WriteOp,
) -> anyhow::Result<()> {
    storage::write_loop_state_with_op(config, state, op)
}

pub fn load_loop_state(config: &Config, loop_id: &str) -> anyhow::Result<LoopState> {
    storage::load_loop_state(config, loop_id)
}

pub fn write_loop_round_record(
    config: &Config,
    record: &LoopRoundRecord,
    op: WriteOp,
) -> anyhow::Result<()> {
    storage::write_loop_round_record(config, record, op)
}

#[cfg(test)]
mod tests {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Err(err) = result else {
            return Err(format!("{context}: expected error containing '{needle}'").into());
        };
        if !err.to_string().contains(needle) {
            return Err(format!("error should contain '{needle}', got: {err}").into());
        }
        Ok(())
    }

    #[test]
    fn test_loop_state_round_trips_state_toml() -> Result<(), Box<dyn std::error::Error>> {
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
    fn test_loop_state_updates_lifecycle_item_status_and_round_count()
    -> Result<(), Box<dyn std::error::Error>> {
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
    fn test_loop_state_rejects_invalid_lifecycle_transition()
    -> Result<(), Box<dyn std::error::Error>> {
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
    fn test_loop_state_rejects_invalid_ids_and_contract_violations()
    -> Result<(), Box<dyn std::error::Error>> {
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
}
