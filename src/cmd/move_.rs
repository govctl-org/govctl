//! Move command implementation for work items.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::{ChecklistStatus, WorkItemStatus};
use crate::parse::{load_work_item, write_work_item};
use crate::ui;
use crate::validate::is_valid_work_transition;
use crate::write::{WriteOp, today};
use std::path::Path;

/// Move work item to new status
pub fn move_item(
    config: &Config,
    file: &Path,
    status: WorkItemStatus,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Find the work item file
    let work_path = if file.is_absolute() || file.exists() {
        file.to_path_buf()
    } else {
        // Try in work directory
        let in_work_dir = config.work_dir().join(file);
        if in_work_dir.exists() {
            in_work_dir
        } else {
            // Try to find by partial name
            find_work_item_by_name(config, &file.to_string_lossy())?
        }
    };

    let mut entry = load_work_item(&work_path)?;

    if !is_valid_work_transition(entry.spec.govctl.status, status) {
        anyhow::bail!(
            "Invalid transition: {} -> {}",
            entry.spec.govctl.status.as_ref(),
            status.as_ref()
        );
    }

    // Validate acceptance criteria before marking done
    if status == WorkItemStatus::Done {
        let pending: Vec<_> = entry
            .spec
            .content
            .acceptance_criteria
            .iter()
            .filter(|c| c.status == ChecklistStatus::Pending)
            .map(|c| c.text.as_str())
            .collect();

        if !pending.is_empty() {
            let list = pending
                .iter()
                .map(|t| format!("  - {t}"))
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!(
                "Cannot mark as done: {} pending acceptance criteria:\n{}",
                pending.len(),
                list
            );
        }
    }

    entry.spec.govctl.status = status;

    // Update dates
    match status {
        WorkItemStatus::Active => {
            if entry.spec.govctl.started.is_none() {
                entry.spec.govctl.started = Some(today());
            }
        }
        WorkItemStatus::Done | WorkItemStatus::Cancelled => {
            entry.spec.govctl.completed = Some(today());
        }
        WorkItemStatus::Queue => {}
    }

    write_work_item(&work_path, &entry.spec, op)?;

    if !op.is_preview() {
        let filename = work_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| work_path.display().to_string());
        ui::moved(&filename, status.as_ref());
    }

    Ok(vec![])
}

/// Find work item by partial name or ID
fn find_work_item_by_name(config: &Config, name: &str) -> anyhow::Result<std::path::PathBuf> {
    use crate::parse::load_work_items;

    // First try: load all work items and match by ID
    if name.starts_with("WI-") {
        let items = load_work_items(config)?;
        if let Some(item) = items.iter().find(|w| w.spec.govctl.id == name) {
            return Ok(item.path.clone());
        }
    }

    // Second try: match by filename
    let work_dir = &config.work_dir();

    if !work_dir.exists() {
        anyhow::bail!("Work directory not found: {}", work_dir.display());
    }

    let entries: Vec<_> = std::fs::read_dir(work_dir)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(name))
                .unwrap_or(false)
        })
        .collect();

    match entries.len() {
        0 => anyhow::bail!("No work item found matching: {name}"),
        1 => Ok(entries[0].path()),
        _ => {
            let names: Vec<_> = entries
                .iter()
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .collect();
            anyhow::bail!("Multiple work items match '{}': {}", name, names.join(", "));
        }
    }
}
