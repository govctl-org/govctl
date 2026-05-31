use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::write::WriteOp;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

mod storage;

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
        if !is_valid_loop_transition(current, next) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1203LoopInvalidTransition,
                format!("Invalid loop transition: {current:?} -> {next:?}"),
                self.loop_meta.id.clone(),
            )
            .into());
        }
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
        validate_loop_id(&self.loop_meta.id)?;
        if let Some(expected) = expected_loop_id
            && self.loop_meta.id != expected
        {
            return Err(invalid_state(
                &self.loop_meta.id,
                format!(
                    "loop.id '{}' does not match loop directory '{}'",
                    self.loop_meta.id, expected
                ),
            ));
        }

        ensure_no_duplicates(
            &self.loop_meta.root_work_items,
            "loop.root_work_items",
            &self.loop_meta.id,
        )?;
        ensure_no_duplicates(
            &self.loop_meta.work_items,
            "loop.work_items",
            &self.loop_meta.id,
        )?;

        let work_items: BTreeSet<&str> = self
            .loop_meta
            .work_items
            .iter()
            .map(String::as_str)
            .collect();
        for work_id in &self.loop_meta.work_items {
            ensure_work_item_id(work_id, &self.loop_meta.id)?;
        }
        for root in &self.loop_meta.root_work_items {
            ensure_work_item_id(root, &self.loop_meta.id)?;
            if !work_items.contains(root.as_str()) {
                return Err(invalid_state(
                    &self.loop_meta.id,
                    format!("root work item '{root}' is missing from loop.work_items"),
                ));
            }
        }

        for work_id in &self.loop_meta.work_items {
            if !self.dependencies.contains_key(work_id) {
                return Err(invalid_state(
                    &self.loop_meta.id,
                    format!("missing dependency entry for work item: {work_id}"),
                ));
            }
            if !self.items.contains_key(work_id) {
                return Err(invalid_state(
                    &self.loop_meta.id,
                    format!("missing item state for work item: {work_id}"),
                ));
            }
        }

        for (work_id, dependencies) in &self.dependencies {
            if !work_items.contains(work_id.as_str()) {
                return Err(invalid_state(
                    &self.loop_meta.id,
                    format!("dependency entry '{work_id}' is not in loop.work_items"),
                ));
            }
            ensure_no_duplicates(
                dependencies,
                &format!("dependencies.{work_id}"),
                &self.loop_meta.id,
            )?;
            for dependency in dependencies {
                ensure_work_item_id(dependency, &self.loop_meta.id)?;
                if !work_items.contains(dependency.as_str()) {
                    return Err(invalid_state(
                        &self.loop_meta.id,
                        format!(
                            "dependency '{dependency}' for '{work_id}' is missing from loop.work_items"
                        ),
                    ));
                }
            }
        }

        for work_id in self.items.keys() {
            if !work_items.contains(work_id.as_str()) {
                return Err(invalid_state(
                    &self.loop_meta.id,
                    format!("item state '{work_id}' is not in loop.work_items"),
                ));
            }
        }

        Ok(())
    }
}

impl LoopRoundRecord {
    pub fn validate(&self) -> anyhow::Result<()> {
        validate_loop_id(&self.loop_id)?;
        ensure_work_item_id(&self.work_item_id, &self.loop_id)?;
        if self.round_number == 0 {
            return Err(invalid_state(
                &self.loop_id,
                "loop round record round_number must be at least 1",
            ));
        }
        if self.max_rounds == 0 {
            return Err(invalid_state(
                &self.loop_id,
                "loop round record max_rounds must be at least 1",
            ));
        }
        if self.round_number > self.max_rounds {
            return Err(invalid_state(
                &self.loop_id,
                format!(
                    "loop round record round_number {} exceeds max_rounds {}",
                    self.round_number, self.max_rounds
                ),
            ));
        }
        ensure_loop_item_status(
            &self.item_status_before,
            "item_status_before",
            &self.loop_id,
        )?;
        ensure_loop_item_status(&self.item_status_after, "item_status_after", &self.loop_id)?;
        ensure_work_status(
            &self.work_status_before,
            "work_status_before",
            &self.loop_id,
        )?;
        ensure_work_status(&self.work_status_after, "work_status_after", &self.loop_id)?;
        ensure_loop_item_status(&self.outcome, "outcome", &self.loop_id)?;
        ensure_non_empty(&self.action, "action", &self.loop_id)?;
        if let Some(reason) = &self.reason {
            ensure_non_empty(reason, "reason", &self.loop_id)?;
        }
        Ok(())
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

pub fn is_valid_loop_transition(from: LoopLifecycleState, to: LoopLifecycleState) -> bool {
    matches!(
        (from, to),
        (LoopLifecycleState::Pending, LoopLifecycleState::Active)
            | (LoopLifecycleState::Active, LoopLifecycleState::Paused)
            | (LoopLifecycleState::Paused, LoopLifecycleState::Active)
            | (LoopLifecycleState::Active, LoopLifecycleState::Completed)
            | (LoopLifecycleState::Active, LoopLifecycleState::Failed)
            | (LoopLifecycleState::Paused, LoopLifecycleState::Failed)
    )
}

pub fn validate_loop_id(loop_id: &str) -> anyhow::Result<()> {
    let valid_first = loop_id
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric());
    let valid_rest = loop_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if loop_id.is_empty()
        || !valid_first
        || !valid_rest
        || loop_id.contains('/')
        || loop_id.contains('\\')
        || loop_id.contains("..")
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E1204LoopInvalidId,
            format!(
                "Invalid loop ID '{loop_id}': must match ^[A-Za-z0-9][A-Za-z0-9._-]*$ and must not contain path traversal"
            ),
            loop_id,
        )
        .into());
    }
    Ok(())
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

fn ensure_work_item_id(work_id: &str, loop_id: &str) -> anyhow::Result<()> {
    if crate::validate::is_work_item_id(work_id) {
        Ok(())
    } else {
        Err(invalid_state(
            loop_id,
            format!("invalid work item ID in loop state: {work_id}"),
        ))
    }
}

fn ensure_no_duplicates(values: &[String], field: &str, loop_id: &str) -> anyhow::Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.as_str()) {
            return Err(invalid_state(
                loop_id,
                format!("duplicate value '{value}' in {field}"),
            ));
        }
    }
    Ok(())
}

fn invalid_state(loop_id: &str, message: impl Into<String>) -> anyhow::Error {
    Diagnostic::new(DiagnosticCode::E1201LoopStateInvalid, message, loop_id).into()
}

fn ensure_loop_item_status(value: &str, field: &str, loop_id: &str) -> anyhow::Result<()> {
    if matches!(
        value,
        "pending" | "active" | "done" | "failed" | "blocked" | "cancelled"
    ) {
        Ok(())
    } else {
        Err(invalid_state(
            loop_id,
            format!("invalid loop round record {field}: {value}"),
        ))
    }
}

fn ensure_work_status(value: &str, field: &str, loop_id: &str) -> anyhow::Result<()> {
    if matches!(value, "queue" | "active" | "done" | "cancelled") {
        Ok(())
    } else {
        Err(invalid_state(
            loop_id,
            format!("invalid loop round record {field}: {value}"),
        ))
    }
}

fn ensure_non_empty(value: &str, field: &str, loop_id: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        Err(invalid_state(
            loop_id,
            format!("loop round record {field} must not be empty"),
        ))
    } else {
        Ok(())
    }
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
