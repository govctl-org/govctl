use super::super::engine as edit_engine;
use super::super::path::FieldPath;
use crate::config::Config;
use crate::model::{ProjectIndex, WorkItemEntry};

pub(in crate::cmd::edit) fn is_work_dependency_target(
    target: &edit_engine::ResolvedTarget,
) -> bool {
    match target {
        edit_engine::ResolvedTarget::Node { path, .. } => is_work_dependency_path(path),
        edit_engine::ResolvedTarget::IndexedItem { container_path, .. } => {
            is_work_dependency_path(container_path)
        }
    }
}

pub(in crate::cmd::edit) fn validate_work_dependency_edit(
    config: &Config,
    entry: &WorkItemEntry,
) -> anyhow::Result<()> {
    let mut index = ProjectIndex {
        work_items: crate::parse::load_work_items(config)?,
        ..Default::default()
    };

    let mut replaced = false;
    for work in &mut index.work_items {
        if work.spec.govctl.id == entry.spec.govctl.id {
            *work = entry.clone();
            replaced = true;
            break;
        }
    }
    if !replaced {
        index.work_items.push(entry.clone());
    }

    if let Some(diagnostic) = crate::validate::validate_work_dependencies(&index, config)
        .into_iter()
        .next()
    {
        return Err(diagnostic.into());
    }

    Ok(())
}

fn is_work_dependency_path(path: &FieldPath) -> bool {
    path.as_simple() == Some("depends_on") || path.to_string() == "govctl.depends_on"
}
