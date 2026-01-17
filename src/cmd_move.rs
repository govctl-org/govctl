//! Move command implementation for work items.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::WorkItemStatus;
use crate::parse::{load_work_item, write_work_item};
use crate::validate::is_valid_work_transition;
use crate::write::today;
use std::path::Path;

/// Move work item to new status
pub fn move_item(
    config: &Config,
    file: &Path,
    status: WorkItemStatus,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Find the work item file
    let work_path = if file.is_absolute() || file.exists() {
        file.to_path_buf()
    } else {
        // Try in work directory
        let in_work_dir = config.paths.work_dir.join(file);
        if in_work_dir.exists() {
            in_work_dir
        } else {
            // Try to find by partial name
            find_work_item_by_name(config, &file.to_string_lossy())?
        }
    };

    let mut entry = load_work_item(&work_path)?;

    if !is_valid_work_transition(entry.spec.phaseos.status, status) {
        anyhow::bail!(
            "Invalid transition: {} -> {}",
            entry.spec.phaseos.status.as_ref(),
            status.as_ref()
        );
    }

    entry.spec.phaseos.status = status;

    // Update dates
    match status {
        WorkItemStatus::Active => {
            if entry.spec.phaseos.start_date.is_none() {
                entry.spec.phaseos.start_date = Some(today());
            }
        }
        WorkItemStatus::Done | WorkItemStatus::Cancelled => {
            entry.spec.phaseos.done_date = Some(today());
        }
        WorkItemStatus::Queue => {}
    }

    write_work_item(&work_path, &entry.spec)?;

    eprintln!(
        "Moved {} to {}",
        work_path.file_name().unwrap().to_string_lossy(),
        status.as_ref()
    );

    Ok(vec![])
}

/// Find work item by partial name
fn find_work_item_by_name(config: &Config, name: &str) -> anyhow::Result<std::path::PathBuf> {
    let work_dir = &config.paths.work_dir;

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
