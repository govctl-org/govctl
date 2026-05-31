//! Loop command surface for local execution state.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_planner::{
    build_loop_plan_from_config, propagate_blocked_outcomes, topological_order_for_state,
};
use crate::loop_state::{
    LoopLifecycleState, LoopRoundRecord, LoopState, LoopWorkItemStatus, load_loop_state,
    loop_state_path, loop_state_root, validate_loop_id, write_loop_round_record,
    write_loop_state_with_op,
};
use crate::model::{ChecklistStatus, WorkItemEntry, WorkItemStatus};
use crate::write::WriteOp;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

pub fn start(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    ensure_root_work_items(root_work_items)?;

    if let Some(existing) = find_reusable_loop(config, loop_id, root_work_items)? {
        print_loop("Reused", &existing)?;
        return Ok(vec![]);
    }

    let loop_id = loop_id
        .map(str::to_string)
        .unwrap_or_else(|| generated_loop_id(root_work_items));
    validate_loop_id(&loop_id)?;

    let plan = build_loop_plan_from_config(config, &loop_id, root_work_items)?;
    write_loop_state_with_op(config, &plan.state, op)?;
    let verb = if op.is_preview() {
        "Would start"
    } else {
        "Started"
    };
    print_loop(verb, &plan.state)?;
    Ok(vec![])
}

pub fn show(config: &Config, loop_id: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let state = load_loop_state(config, loop_id)?;
    print_loop("Loop", &state)?;
    Ok(vec![])
}

pub fn resume(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
) -> anyhow::Result<Vec<Diagnostic>> {
    let state = if let Some(loop_id) = loop_id {
        let state = load_loop_state(config, loop_id)?;
        if !root_work_items.is_empty() {
            ensure_root_work_items(root_work_items)?;
            ensure_same_root_set(&state, root_work_items)?;
        }
        state
    } else {
        ensure_root_work_items(root_work_items)?;
        find_matching_non_terminal_loop(config, root_work_items)?.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1207LoopResumeNotFound,
                "No matching non-terminal loop state found; start a new loop or pass --id",
                root_work_items.join(", "),
            )
        })?
    };

    print_loop("Resumed", &state)?;
    Ok(vec![])
}

pub fn run(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
    max_rounds: u32,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    if max_rounds == 0 {
        return Err(Diagnostic::new(
            DiagnosticCode::E1211LoopInvalidMaxRounds,
            "Loop max rounds must be at least 1",
            "loop",
        )
        .into());
    }

    let mut state = state_for_run(config, loop_id, root_work_items)?;
    ensure_loop_can_run(&state)?;

    println!("Running loop {}", state.loop_meta.id);
    println!("Max rounds: {max_rounds}");

    enter_active_state(&mut state)?;
    write_loop_state_with_op(config, &state, op)?;

    let mut failures = Vec::new();
    execute_run_round(config, &mut state, max_rounds, op, &mut failures)?;
    finalize_run_state(&mut state)?;
    write_loop_state_with_op(config, &state, op)?;

    match state.loop_meta.state {
        LoopLifecycleState::Completed => {
            print_loop("Completed", &state)?;
            Ok(vec![])
        }
        LoopLifecycleState::Paused => {
            print_loop("Paused", &state)?;
            Ok(vec![])
        }
        LoopLifecycleState::Failed => {
            print_loop("Failed", &state)?;
            Err(Diagnostic::new(
                DiagnosticCode::E1210LoopExecutionFailed,
                loop_failure_message(&state, &failures),
                state.loop_meta.id.clone(),
            )
            .into())
        }
        LoopLifecycleState::Pending | LoopLifecycleState::Active => Ok(vec![]),
    }
}

fn state_for_run(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
) -> anyhow::Result<LoopState> {
    if let Some(loop_id) = loop_id {
        match load_loop_state(config, loop_id) {
            Ok(state) => {
                if !root_work_items.is_empty() {
                    ensure_root_work_items(root_work_items)?;
                    ensure_same_root_set(&state, root_work_items)?;
                }
                return Ok(state);
            }
            Err(err) if diagnostic_code(&err) == Some(DiagnosticCode::E1202LoopStateNotFound) => {
                if root_work_items.is_empty() {
                    return Err(err);
                }
            }
            Err(err) => return Err(err),
        }

        ensure_root_work_items(root_work_items)?;
        return Ok(build_loop_plan_from_config(config, loop_id, root_work_items)?.state);
    }

    ensure_root_work_items(root_work_items)?;
    if let Some(state) = find_matching_non_terminal_loop(config, root_work_items)? {
        Ok(state)
    } else {
        let loop_id = generated_loop_id(root_work_items);
        Ok(build_loop_plan_from_config(config, &loop_id, root_work_items)?.state)
    }
}

fn ensure_loop_can_run(state: &LoopState) -> anyhow::Result<()> {
    if matches!(
        state.loop_meta.state,
        LoopLifecycleState::Completed | LoopLifecycleState::Failed
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1210LoopExecutionFailed,
            format!(
                "Cannot run terminal loop '{}' in {} state",
                state.loop_meta.id,
                state.loop_meta.state.as_str()
            ),
            state.loop_meta.id.clone(),
        )
        .into());
    }
    Ok(())
}

fn enter_active_state(state: &mut LoopState) -> anyhow::Result<()> {
    match state.loop_meta.state {
        LoopLifecycleState::Pending | LoopLifecycleState::Paused => {
            state.transition_to(LoopLifecycleState::Active)
        }
        LoopLifecycleState::Active => Ok(()),
        LoopLifecycleState::Completed | LoopLifecycleState::Failed => ensure_loop_can_run(state),
    }
}

fn execute_run_round(
    config: &Config,
    state: &mut LoopState,
    max_rounds: u32,
    op: WriteOp,
    failures: &mut Vec<String>,
) -> anyhow::Result<()> {
    for work_id in topological_order_for_state(state)? {
        propagate_blocked_outcomes(state)?;
        if is_terminal_item(state, &work_id) {
            continue;
        }
        match dependency_readiness(state, &work_id)? {
            DependencyReadiness::Ready => {}
            DependencyReadiness::Waiting => continue,
            DependencyReadiness::Blocked => {
                state.set_item_status(&work_id, LoopWorkItemStatus::Blocked)?;
                continue;
            }
        }

        if let Some(failure) = execute_work_item_round(config, state, &work_id, max_rounds, op)? {
            failures.push(failure);
            propagate_blocked_outcomes(state)?;
        }
        write_loop_state_with_op(config, state, op)?;
    }
    Ok(())
}

fn execute_work_item_round(
    config: &Config,
    state: &mut LoopState,
    work_id: &str,
    max_rounds: u32,
    op: WriteOp,
) -> anyhow::Result<Option<String>> {
    let item_status_before = loop_item_status_string(state, work_id)?;
    let mut entry = load_work_item_by_id(config, work_id)?;
    let work_status_before = work_item_status_string(entry.spec.govctl.status);
    match entry.spec.govctl.status {
        WorkItemStatus::Done => {
            state.set_item_status(work_id, LoopWorkItemStatus::Done)?;
            return Ok(None);
        }
        WorkItemStatus::Cancelled => {
            state.set_item_status(work_id, LoopWorkItemStatus::Cancelled)?;
            return Ok(None);
        }
        WorkItemStatus::Queue => {
            if let Err(err) =
                crate::cmd::move_::move_item(config, Path::new(work_id), WorkItemStatus::Active, op)
            {
                state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
                return Ok(Some(format!(
                    "{work_id}: failed to transition to active: {}",
                    error_summary(&err)
                )));
            }
            if !op.is_preview() {
                entry = load_work_item_by_id(config, work_id)?;
            } else {
                entry.spec.govctl.status = WorkItemStatus::Active;
            }
        }
        WorkItemStatus::Active => {}
    }

    state.set_item_status(work_id, LoopWorkItemStatus::Active)?;
    let current_rounds = state
        .items
        .get(work_id)
        .map(|item| item.round_count)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("missing item state for work item: {work_id}"),
                state.loop_meta.id.clone(),
            )
        })?;
    if current_rounds >= max_rounds {
        state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
        return Ok(Some(format!(
            "{work_id}: maximum rounds reached ({max_rounds})"
        )));
    }
    let round = state.increment_round_count(work_id)?;

    let (action, reason, failure) = if acceptance_criteria_satisfied(&entry) {
        match crate::cmd::move_::move_item(config, Path::new(work_id), WorkItemStatus::Done, op) {
            Ok(_) => {
                state.set_item_status(work_id, LoopWorkItemStatus::Done)?;
                entry.spec.govctl.status = WorkItemStatus::Done;
                (
                    "evaluated acceptance criteria and completed work item".to_string(),
                    None,
                    None,
                )
            }
            Err(err) => {
                let summary = error_summary(&err);
                if round < max_rounds && is_retryable_guard_assertion_failure(&summary) {
                    (
                        "evaluated acceptance criteria and verification guards".to_string(),
                        Some(format!(
                            "verification guard assertion failed; max rounds not reached: {summary}"
                        )),
                        None,
                    )
                } else {
                    state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
                    let reason = format!("failed to complete after round {round}: {summary}");
                    (
                        "evaluated acceptance criteria and verification guards".to_string(),
                        Some(reason.clone()),
                        Some(format!("{work_id}: {reason}")),
                    )
                }
            }
        }
    } else if round >= max_rounds {
        state.set_item_status(work_id, LoopWorkItemStatus::Failed)?;
        let reason =
            format!("maximum rounds reached ({max_rounds}) with pending acceptance criteria");
        (
            "evaluated acceptance criteria".to_string(),
            Some(reason.clone()),
            Some(format!("{work_id}: {reason}")),
        )
    } else {
        (
            "evaluated acceptance criteria".to_string(),
            Some("pending acceptance criteria remain; max rounds not reached".to_string()),
            None,
        )
    };

    let item_status_after = loop_item_status_string(state, work_id)?;
    let work_status_after = if op.is_preview() {
        work_item_status_string(entry.spec.govctl.status)
    } else {
        work_item_status_string(load_work_item_by_id(config, work_id)?.spec.govctl.status)
    };
    write_loop_round_record(
        config,
        &LoopRoundRecord {
            loop_id: state.loop_meta.id.clone(),
            work_item_id: work_id.to_string(),
            round_number: round,
            max_rounds,
            item_status_before,
            item_status_after: item_status_after.clone(),
            work_status_before,
            work_status_after,
            action,
            outcome: item_status_after,
            reason,
        },
        op,
    )?;
    Ok(failure)
}

fn finalize_run_state(state: &mut LoopState) -> anyhow::Result<()> {
    propagate_blocked_outcomes(state)?;
    let has_failed = state.items.values().any(|item| {
        matches!(
            item.status,
            LoopWorkItemStatus::Failed | LoopWorkItemStatus::Blocked
        )
    });
    let all_terminal = state.items.values().all(|item| {
        matches!(
            item.status,
            LoopWorkItemStatus::Done
                | LoopWorkItemStatus::Failed
                | LoopWorkItemStatus::Blocked
                | LoopWorkItemStatus::Cancelled
        )
    });

    if has_failed {
        state.transition_to(LoopLifecycleState::Failed)
    } else if all_terminal {
        state.transition_to(LoopLifecycleState::Completed)
    } else {
        state.transition_to(LoopLifecycleState::Paused)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyReadiness {
    Ready,
    Waiting,
    Blocked,
}

fn dependency_readiness(state: &LoopState, work_id: &str) -> anyhow::Result<DependencyReadiness> {
    let dependencies = state.dependencies.get(work_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            format!("missing dependency entry for work item: {work_id}"),
            state.loop_meta.id.clone(),
        )
    })?;
    if dependencies.iter().any(|dependency| {
        matches!(
            state.items[dependency.as_str()].status,
            LoopWorkItemStatus::Failed
                | LoopWorkItemStatus::Blocked
                | LoopWorkItemStatus::Cancelled
        )
    }) {
        return Ok(DependencyReadiness::Blocked);
    }
    if dependencies
        .iter()
        .all(|dependency| state.items[dependency.as_str()].status == LoopWorkItemStatus::Done)
    {
        Ok(DependencyReadiness::Ready)
    } else {
        Ok(DependencyReadiness::Waiting)
    }
}

fn acceptance_criteria_satisfied(entry: &WorkItemEntry) -> bool {
    !entry.spec.content.acceptance_criteria.is_empty()
        && entry
            .spec
            .content
            .acceptance_criteria
            .iter()
            .all(|criterion| criterion.status != ChecklistStatus::Pending)
}

fn is_terminal_item(state: &LoopState, work_id: &str) -> bool {
    matches!(
        state.items[work_id].status,
        LoopWorkItemStatus::Done
            | LoopWorkItemStatus::Failed
            | LoopWorkItemStatus::Blocked
            | LoopWorkItemStatus::Cancelled
    )
}

fn loop_item_status_string(state: &LoopState, work_id: &str) -> anyhow::Result<String> {
    state
        .items
        .get(work_id)
        .map(|item| item.status.as_str().to_string())
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("missing item state for work item: {work_id}"),
                state.loop_meta.id.clone(),
            )
            .into()
        })
}

fn work_item_status_string(status: WorkItemStatus) -> String {
    match status {
        WorkItemStatus::Queue => "queue",
        WorkItemStatus::Active => "active",
        WorkItemStatus::Done => "done",
        WorkItemStatus::Cancelled => "cancelled",
    }
    .to_string()
}

fn load_work_item_by_id(config: &Config, work_id: &str) -> anyhow::Result<WorkItemEntry> {
    crate::parse::load_work_items(config)?
        .into_iter()
        .find(|entry| entry.spec.govctl.id == work_id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {work_id}"),
                work_id,
            )
            .into()
        })
}

fn loop_failure_message(state: &LoopState, failures: &[String]) -> String {
    if failures.is_empty() {
        format!("Loop '{}' failed", state.loop_meta.id)
    } else {
        format!(
            "Loop '{}' failed:\n{}",
            state.loop_meta.id,
            failures
                .iter()
                .map(|failure| format!("  - {failure}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn error_summary(err: &anyhow::Error) -> String {
    if let Some(diagnostic) = err.downcast_ref::<Diagnostic>() {
        diagnostic.message.clone()
    } else {
        err.to_string()
    }
}

fn is_retryable_guard_assertion_failure(summary: &str) -> bool {
    summary.contains("verification guard requirements failed")
        && summary.contains("Verification guard '")
        && summary.contains(" failed")
        && !summary.contains("timed out")
        && !summary.contains("Failed to start")
        && !summary.contains("Unknown verification guard")
}

fn find_reusable_loop(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
) -> anyhow::Result<Option<LoopState>> {
    if let Some(loop_id) = loop_id {
        match load_loop_state(config, loop_id) {
            Ok(state) => {
                ensure_same_root_set(&state, root_work_items)?;
                return Ok(Some(state));
            }
            Err(err) if diagnostic_code(&err) == Some(DiagnosticCode::E1202LoopStateNotFound) => {
                return Ok(None);
            }
            Err(err) => return Err(err),
        }
    }

    find_matching_non_terminal_loop(config, root_work_items)
}

fn find_matching_non_terminal_loop(
    config: &Config,
    root_work_items: &[String],
) -> anyhow::Result<Option<LoopState>> {
    let root = loop_state_root(config);
    if !root.exists() {
        return Ok(None);
    }

    let mut matches = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to read loop state directory: {e}"),
            root.display().to_string(),
        )
    })? {
        let entry = entry.map_err(|e| {
            Diagnostic::new(
                DiagnosticCode::E0901IoError,
                format!("Failed to read loop state entry: {e}"),
                root.display().to_string(),
            )
        })?;
        if !entry.path().is_dir() {
            continue;
        }
        let Some(loop_id) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let state_path = loop_state_path(config, &loop_id)?;
        if !state_path.exists() {
            continue;
        }
        let state = load_loop_state(config, &loop_id)?;
        if is_non_terminal(state.loop_meta.state)
            && same_root_set(&state.loop_meta.root_work_items, root_work_items)
        {
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
            root_work_items.join(", "),
        )
        .into()),
    }
}

fn print_loop(verb: &str, state: &LoopState) -> anyhow::Result<()> {
    if verb == "Loop" {
        println!("Loop {}", state.loop_meta.id);
    } else {
        println!("{} loop {}", verb, state.loop_meta.id);
    }
    println!("State: {}", state.loop_meta.state.as_str());
    println!("Roots: {}", state.loop_meta.root_work_items.join(", "));
    println!(
        "Resolved: {} work item(s)",
        state.loop_meta.work_items.len()
    );
    println!("Plan:");
    for (index, work_id) in topological_order_for_state(state)?.iter().enumerate() {
        let item = state.items.get(work_id).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("missing item state for work item: {work_id}"),
                state.loop_meta.id.clone(),
            )
        })?;
        let deps = state
            .dependencies
            .get(work_id)
            .filter(|deps| !deps.is_empty())
            .map(|deps| deps.join(","))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {}. {} status={} rounds={} depends_on={}",
            index + 1,
            work_id,
            item.status.as_str(),
            item.round_count,
            deps
        );
    }
    Ok(())
}

fn ensure_root_work_items(root_work_items: &[String]) -> anyhow::Result<()> {
    if root_work_items.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "At least one root work item ID is required",
            "loop",
        )
        .into());
    }
    let mut seen = BTreeSet::new();
    for work_id in root_work_items {
        if !crate::validate::is_work_item_id(work_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0409WorkDependencyInvalid,
                format!("Loop root '{work_id}' must be a work item ID"),
                "loop",
            )
            .into());
        }
        if !seen.insert(work_id.as_str()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("duplicate loop root work item: {work_id}"),
                "loop",
            )
            .into());
        }
    }
    Ok(())
}

fn ensure_same_root_set(state: &LoopState, root_work_items: &[String]) -> anyhow::Result<()> {
    if same_root_set(&state.loop_meta.root_work_items, root_work_items) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1209LoopRootMismatch,
            format!(
                "Loop root work item set does not match existing loop state: stored [{}], requested [{}]",
                state.loop_meta.root_work_items.join(", "),
                root_work_items.join(", ")
            ),
            state.loop_meta.id.clone(),
        )
        .into())
    }
}

fn same_root_set(left: &[String], right: &[String]) -> bool {
    left.iter().collect::<BTreeSet<_>>() == right.iter().collect::<BTreeSet<_>>()
}

fn is_non_terminal(state: LoopLifecycleState) -> bool {
    matches!(
        state,
        LoopLifecycleState::Pending | LoopLifecycleState::Active | LoopLifecycleState::Paused
    )
}

fn generated_loop_id(root_work_items: &[String]) -> String {
    let mut roots = root_work_items.to_vec();
    roots.sort();
    let mut hasher = Sha256::new();
    hasher.update(roots.join("|").as_bytes());
    let digest = hasher.finalize();
    let short = digest
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "loop-{}-{short}",
        chrono::Local::now().format("%Y%m%d%H%M%S")
    )
}

fn diagnostic_code(err: &anyhow::Error) -> Option<DiagnosticCode> {
    err.downcast_ref::<Diagnostic>()
        .map(|diagnostic| diagnostic.code)
}
