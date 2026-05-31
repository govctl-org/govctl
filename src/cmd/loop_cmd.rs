//! Loop command surface for local execution state.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_planner::{build_loop_plan_from_config, topological_order_for_state};
use crate::loop_state::{
    LoopLifecycleState, LoopState, load_loop_state, loop_state_path, loop_state_root,
    validate_loop_id, write_loop_state_with_op,
};
use crate::write::WriteOp;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

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
