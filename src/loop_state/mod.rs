use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod storage;
#[cfg(test)]
mod tests;
mod validation;

pub use storage::{
    load_loop_round_record, load_loop_state, loop_round_path, loop_state_path, loop_state_root,
    write_loop_round_record, write_loop_state_with_op,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum LoopNextAction {
    #[default]
    Start,
    WriteSummary,
    Continue,
    ResolveBlocker,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopRoundStatus {
    Open,
    Submitted,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopMeta {
    pub id: String,
    pub state: LoopLifecycleState,
    pub work: Vec<String>,
    pub resolved: Vec<String>,
    #[serde(default)]
    pub current_round: u32,
    #[serde(default)]
    pub next_action: LoopNextAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopItemState {
    pub status: LoopWorkItemStatus,
    pub round_count: u32,
    #[serde(default)]
    pub last_round: u32,
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
#[serde(deny_unknown_fields)]
pub struct LoopRoundMeta {
    pub loop_id: String,
    pub round_number: u32,
    pub max_rounds: u32,
    pub status: LoopRoundStatus,
    pub work: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopRoundSummary {
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub verification: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub note_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopRoundRecord {
    #[serde(rename = "round")]
    pub round_meta: LoopRoundMeta,
    #[serde(default)]
    pub summary: LoopRoundSummary,
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
                        last_round: 0,
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
                current_round: 0,
                next_action: LoopNextAction::Start,
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

    #[cfg(test)]
    pub fn increment_round_count(&mut self, work_id: &str) -> DiagnosticResult<u32> {
        let round_number = if self.loop_meta.current_round == 0 {
            self.items
                .get(work_id)
                .map(|item| item.round_count.saturating_add(1))
                .unwrap_or(1)
        } else {
            self.loop_meta.current_round
        };
        self.record_item_round(work_id, round_number)
    }

    pub fn record_item_round(&mut self, work_id: &str, round_number: u32) -> DiagnosticResult<u32> {
        if round_number == 0 {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                "Loop item last_round must be at least 1 when recording a round",
                self.loop_meta.id.clone(),
            ));
        }
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
        item.last_round = round_number;
        Ok(item.round_count)
    }

    pub fn validate(&self, expected_loop_id: Option<&str>) -> DiagnosticResult<()> {
        validation::validate_loop_state(self, expected_loop_id)
    }
}

impl LoopRoundRecord {
    pub fn open(
        loop_id: impl Into<String>,
        round_number: u32,
        max_rounds: u32,
        work: Vec<String>,
    ) -> Self {
        Self {
            round_meta: LoopRoundMeta {
                loop_id: loop_id.into(),
                round_number,
                max_rounds,
                status: LoopRoundStatus::Open,
                work,
            },
            summary: LoopRoundSummary::default(),
        }
    }

    pub fn validate(&self) -> DiagnosticResult<()> {
        validation::validate_loop_round_record(self)
    }

    pub fn has_required_summary_evidence(&self) -> bool {
        self.summary.has_required_evidence()
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

impl LoopNextAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::WriteSummary => "write_summary",
            Self::Continue => "continue",
            Self::ResolveBlocker => "resolve_blocker",
            Self::Complete => "complete",
        }
    }
}

impl LoopRoundSummary {
    pub fn has_blockers(&self) -> bool {
        has_non_empty_value(&self.blockers)
    }

    fn has_required_evidence(&self) -> bool {
        has_non_empty_value(&self.actions)
            && has_non_empty_value(&self.changed_paths)
            && (has_non_empty_value(&self.verification) || has_non_empty_value(&self.blockers))
    }
}

fn has_non_empty_value(values: &[String]) -> bool {
    values.iter().any(|value| !value.trim().is_empty())
}
