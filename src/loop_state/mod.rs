use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::write::WriteOp;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

mod storage;
#[cfg(test)]
mod tests;
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
#[serde(deny_unknown_fields)]
pub struct LoopMeta {
    pub id: String,
    pub state: LoopLifecycleState,
    pub work: Vec<String>,
    pub resolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopItemState {
    pub status: LoopWorkItemStatus,
    pub round_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
        work: Vec<String>,
        resolved: Vec<String>,
        dependencies: BTreeMap<String, Vec<String>>,
    ) -> DiagnosticResult<Self> {
        let items = resolved
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
                work,
                resolved,
            },
            dependencies,
            items,
        };
        state.validate(None)?;
        Ok(state)
    }

    pub fn transition_to(&mut self, next: LoopLifecycleState) -> DiagnosticResult<()> {
        let current = self.loop_meta.state;
        validation::validate_loop_transition(&self.loop_meta.id, current, next)?;
        self.loop_meta.state = next;
        Ok(())
    }

    pub fn set_item_status(
        &mut self,
        work_id: &str,
        status: LoopWorkItemStatus,
    ) -> DiagnosticResult<()> {
        let Some(item) = self.items.get_mut(work_id) else {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("Loop state has no item entry for work item: {work_id}"),
                self.loop_meta.id.clone(),
            ));
        };
        item.status = status;
        Ok(())
    }

    pub fn increment_round_count(&mut self, work_id: &str) -> DiagnosticResult<u32> {
        let Some(item) = self.items.get_mut(work_id) else {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("Loop state has no item entry for work item: {work_id}"),
                self.loop_meta.id.clone(),
            ));
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

    pub fn validate(&self, expected_loop_id: Option<&str>) -> DiagnosticResult<()> {
        validation::validate_loop_state(self, expected_loop_id)
    }
}

impl LoopRoundRecord {
    pub fn validate(&self) -> DiagnosticResult<()> {
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

pub fn loop_state_path(config: &Config, loop_id: &str) -> DiagnosticResult<PathBuf> {
    storage::loop_state_path(config, loop_id)
}

pub fn loop_state_root(config: &Config) -> PathBuf {
    storage::loop_state_root(config)
}

pub fn write_loop_state_with_op(
    config: &Config,
    state: &LoopState,
    op: WriteOp,
) -> DiagnosticResult<()> {
    storage::write_loop_state_with_op(config, state, op)
}

pub fn load_loop_state(config: &Config, loop_id: &str) -> DiagnosticResult<LoopState> {
    storage::load_loop_state(config, loop_id)
}

pub fn write_loop_round_record(
    config: &Config,
    record: &LoopRoundRecord,
    op: WriteOp,
) -> DiagnosticResult<()> {
    storage::write_loop_round_record(config, record, op)
}
