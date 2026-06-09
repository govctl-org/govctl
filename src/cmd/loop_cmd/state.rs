use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_planner::replan_loop_state;
use crate::loop_state::{
    LoopItemState, LoopLifecycleState, LoopState, load_loop_state, loop_state_path,
    loop_state_root, validate_loop_id,
};
use crate::model::WorkItemEntry;
use std::collections::BTreeSet;

pub(super) fn find_reusable_loop(
    config: &Config,
    loop_id: Option<&str>,
    work: &[String],
) -> DiagnosticResult<Option<LoopState>> {
    if let Some(loop_id) = loop_id {
        match load_loop_state(config, loop_id) {
            Ok(state) => {
                ensure_same_work_set(&state, work)?;
                return Ok(Some(state));
            }
            Err(err) if err.code == DiagnosticCode::E1202LoopStateNotFound => return Ok(None),
            Err(err) => return Err(err),
        }
    }

    find_matching_non_terminal_loop(config, work)
}

fn find_matching_non_terminal_loop(
    config: &Config,
    work: &[String],
) -> DiagnosticResult<Option<LoopState>> {
    let mut matches = Vec::new();
    for loop_id in canonical_loop_ids(config)? {
        let state_path = loop_state_path(config, &loop_id)?;
        if !state_path.exists() {
            continue;
        }
        let state = load_loop_state(config, &loop_id)?;
        if is_non_terminal(state.loop_meta.state) && same_work_set(&state.loop_meta.work, work) {
            matches.push(state);
        }
    }

    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => Err(Diagnostic::new(
            DiagnosticCode::E1208LoopResumeAmbiguous,
            format!(
                "Multiple matching non-terminal loops found: {}",
                matches
                    .iter()
                    .map(|state| state.loop_meta.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            work.join(", "),
        )),
    }
}

pub(super) fn canonical_loop_ids(config: &Config) -> DiagnosticResult<Vec<String>> {
    let root = loop_state_root(config);
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut loop_ids = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|e| {
        Diagnostic::io_error("read loop state directory", e, root.display().to_string())
    })? {
        let entry = entry.map_err(|e| {
            Diagnostic::io_error("read loop state entry", e, root.display().to_string())
        })?;
        if !entry.path().is_dir() {
            continue;
        }
        let Some(loop_id) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if validate_loop_id(&loop_id).is_ok() {
            loop_ids.push(loop_id);
        }
    }
    loop_ids.sort();
    Ok(loop_ids)
}

pub(super) fn ensure_work_values(work: &[String]) -> DiagnosticResult<()> {
    if work.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "At least one loop work item ID is required",
            "loop",
        ));
    }
    ensure_unique_work_item_ids(work, "Loop work field value", "loop work item", "loop")
}

pub(super) fn ensure_unique_work_item_ids(
    work: &[String],
    invalid_subject: &str,
    duplicate_subject: &str,
    scope: &str,
) -> DiagnosticResult<()> {
    let mut seen = BTreeSet::new();
    for work_id in work {
        if !crate::validate::is_work_item_id(work_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0409WorkDependencyInvalid,
                format!("{invalid_subject} '{work_id}' must be a work item ID"),
                scope,
            ));
        }
        if !seen.insert(work_id.as_str()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("duplicate {duplicate_subject}: {work_id}"),
                scope,
            ));
        }
    }
    Ok(())
}

fn ensure_same_work_set(state: &LoopState, work: &[String]) -> DiagnosticResult<()> {
    if same_work_set(&state.loop_meta.work, work) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1209LoopWorkMismatch,
            format!(
                "Loop work field does not match existing loop state: stored [{}], requested [{}]",
                state.loop_meta.work.join(", "),
                work.join(", ")
            ),
            state.loop_meta.id.clone(),
        ))
    }
}

pub(super) fn loop_dependencies<'a>(
    state: &'a LoopState,
    work_id: &str,
    subject: &str,
) -> DiagnosticResult<&'a [String]> {
    state
        .dependencies
        .get(work_id)
        .map(Vec::as_slice)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("missing dependency entry for {subject}: {work_id}"),
                state.loop_meta.id.clone(),
            )
        })
}

pub(super) fn loop_item_state<'a>(
    state: &'a LoopState,
    work_id: &str,
) -> DiagnosticResult<&'a LoopItemState> {
    state.items.get(work_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            format!("missing item state for work item: {work_id}"),
            state.loop_meta.id.clone(),
        )
    })
}

pub(super) fn ensure_loop_not_terminal(state: &LoopState, action: &str) -> DiagnosticResult<()> {
    if matches!(
        state.loop_meta.state,
        LoopLifecycleState::Completed | LoopLifecycleState::Failed
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1210LoopExecutionFailed,
            format!(
                "Cannot {action} terminal loop '{}' in {} state",
                state.loop_meta.id,
                state.loop_meta.state.as_str()
            ),
            state.loop_meta.id.clone(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LoopPlanStatus {
    Fresh,
    Stale,
}

impl LoopPlanStatus {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
        }
    }
}

pub(super) fn loop_plan_status(config: &Config, state: &LoopState) -> LoopPlanStatus {
    if current_plan_matches_state(config, state).unwrap_or(false) {
        LoopPlanStatus::Fresh
    } else {
        LoopPlanStatus::Stale
    }
}

pub(super) fn loop_plan_status_from_work_items(
    state: &LoopState,
    all_work_items: &[WorkItemEntry],
) -> LoopPlanStatus {
    if current_plan_matches_work_items(state, all_work_items).unwrap_or(false) {
        LoopPlanStatus::Fresh
    } else {
        LoopPlanStatus::Stale
    }
}

pub(super) fn ensure_loop_plan_fresh(config: &Config, state: &LoopState) -> DiagnosticResult<()> {
    match current_plan_matches_state(config, state) {
        Ok(true) => Ok(()),
        Ok(false) => Err(stale_loop_plan_diagnostic(
            state,
            "stored dependency closure no longer matches current Work Item files",
        )),
        Err(err) => Err(stale_loop_plan_diagnostic(
            state,
            format!(
                "current Work Item dependency closure cannot be resolved ({})",
                err.message
            ),
        )),
    }
}

// Implements [[RFC-0006:C-DEPENDENCY-SEMANTICS]] and
// [[RFC-0006:C-LOOP-SCOPE-MUTATION]] by treating stored scope as a cached plan.
fn current_plan_matches_state(config: &Config, state: &LoopState) -> DiagnosticResult<bool> {
    let all_work_items = crate::parse::load_work_items(config)?;
    current_plan_matches_work_items(state, &all_work_items)
}

fn current_plan_matches_work_items(
    state: &LoopState,
    all_work_items: &[WorkItemEntry],
) -> DiagnosticResult<bool> {
    let plan = replan_loop_state(state, &state.loop_meta.work, all_work_items)?;
    Ok(plan.state.loop_meta.resolved == state.loop_meta.resolved
        && plan.state.dependencies == state.dependencies)
}

fn stale_loop_plan_diagnostic(state: &LoopState, reason: impl AsRef<str>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1201LoopStateInvalid,
        format!(
            "Loop '{}' is stale: {}. Run `govctl loop replan {}` before opening another round.",
            state.loop_meta.id,
            reason.as_ref(),
            state.loop_meta.id
        ),
        state.loop_meta.id.clone(),
    )
}

pub(super) fn generated_loop_id(config: &Config) -> DiagnosticResult<String> {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    generated_loop_id_for_date(config, &date)
}

fn same_work_set(left: &[String], right: &[String]) -> bool {
    left.iter().collect::<BTreeSet<_>>() == right.iter().collect::<BTreeSet<_>>()
}

fn is_non_terminal(state: LoopLifecycleState) -> bool {
    matches!(
        state,
        LoopLifecycleState::Pending | LoopLifecycleState::Active | LoopLifecycleState::Paused
    )
}

fn generated_loop_id_for_date(config: &Config, date: &str) -> DiagnosticResult<String> {
    for sequence in 1..=999 {
        let loop_id = format!("LOOP-{date}-{sequence:03}");
        validate_loop_id(&loop_id)?;
        if !loop_state_root(config).join(&loop_id).exists() {
            return Ok(loop_id);
        }
    }
    Err(Diagnostic::new(
        DiagnosticCode::E1204LoopInvalidId,
        format!("No available loop ID sequence for date {date}"),
        date,
    ))
}
