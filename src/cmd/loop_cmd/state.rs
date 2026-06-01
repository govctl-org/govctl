use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::loop_state::{
    LoopLifecycleState, LoopState, load_loop_state, loop_state_path, loop_state_root,
    validate_loop_id,
};
use std::collections::BTreeSet;

pub(super) fn find_reusable_loop(
    config: &Config,
    loop_id: Option<&str>,
    root_work_items: &[String],
) -> DiagnosticResult<Option<LoopState>> {
    if let Some(loop_id) = loop_id {
        match load_loop_state(config, loop_id) {
            Ok(state) => {
                ensure_same_root_set(&state, root_work_items)?;
                return Ok(Some(state));
            }
            Err(err) if diagnostic_code(&err) == DiagnosticCode::E1202LoopStateNotFound => {
                return Ok(None);
            }
            Err(err) => return Err(err),
        }
    }

    find_matching_non_terminal_loop(config, root_work_items)
}

pub(super) fn find_matching_non_terminal_loop(
    config: &Config,
    root_work_items: &[String],
) -> DiagnosticResult<Option<LoopState>> {
    let mut matches = Vec::new();
    for loop_id in canonical_loop_ids(config)? {
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

pub(super) fn ensure_root_work_items(root_work_items: &[String]) -> DiagnosticResult<()> {
    if root_work_items.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "At least one loop work item ID is required",
            "loop",
        ));
    }
    let mut seen = BTreeSet::new();
    for work_id in root_work_items {
        if !crate::validate::is_work_item_id(work_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0409WorkDependencyInvalid,
                format!("Loop work field value '{work_id}' must be a work item ID"),
                "loop",
            ));
        }
        if !seen.insert(work_id.as_str()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1201LoopStateInvalid,
                format!("duplicate loop work item: {work_id}"),
                "loop",
            ));
        }
    }
    Ok(())
}

pub(super) fn ensure_same_root_set(
    state: &LoopState,
    root_work_items: &[String],
) -> DiagnosticResult<()> {
    if same_root_set(&state.loop_meta.root_work_items, root_work_items) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1209LoopWorkMismatch,
            format!(
                "Loop work field does not match existing loop state: stored [{}], requested [{}]",
                state.loop_meta.root_work_items.join(", "),
                root_work_items.join(", ")
            ),
            state.loop_meta.id.clone(),
        ))
    }
}

pub(super) fn generated_loop_id(config: &Config) -> DiagnosticResult<String> {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    generated_loop_id_for_date(config, &date)
}

pub(super) fn diagnostic_code(err: &Diagnostic) -> DiagnosticCode {
    err.code
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
