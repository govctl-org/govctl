use super::ArtifactType;
use super::adapter::{TomlAdapter, WorkTomlAdapter};
use super::engine as edit_engine;
use super::matching::{MatchOptions, MatchUse, resolve_match_indices};
use super::path::FieldPath;
use super::runtime as edit_runtime;
use super::target_doc::{notify_removed, remove_target_from_doc};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{
    AdrEntry, AdrSpec, GuardEntry, GuardSpec, ProjectIndex, WorkItemEntry, WorkItemSpec,
};
use crate::write::WriteOp;

const TICK_NESTED_PATH_ERROR: &str =
    "tick only supports checklist root paths or indexed checklist items";

pub(super) trait TomlEditableEntry {
    type Spec: serde::Serialize + serde::de::DeserializeOwned;
    fn spec(&self) -> &Self::Spec;
    fn spec_mut(&mut self) -> &mut Self::Spec;
}

impl TomlEditableEntry for AdrEntry {
    type Spec = AdrSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}

impl TomlEditableEntry for WorkItemEntry {
    type Spec = WorkItemSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}

impl TomlEditableEntry for GuardEntry {
    type Spec = GuardSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}

pub(super) fn get_toml_field<A>(
    config: &Config,
    id: &str,
    target: Option<&edit_engine::ResolvedTarget>,
    artifact: ArtifactType,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let entry = A::load(config, id)?;
    if let Some(target) = target {
        let doc = serde_json::to_value(entry.spec())?;
        println!("{}", render_resolved_target(artifact, &doc, target, id)?);
    } else {
        println!("{}", toml::to_string_pretty(entry.spec())?);
    }
    Ok(())
}

pub(super) fn set_toml_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    allow_forced_simple_set: bool,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    apply_toml_target_to_entry(
        &mut entry,
        target,
        value,
        artifact,
        allow_forced_simple_set,
        id,
    )?;
    A::write(config, &entry, op)?;
    Ok(())
}

pub(super) fn set_work_toml_field(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> anyhow::Result<()> {
    let mut entry = WorkTomlAdapter::load(config, id)?;
    apply_toml_target_to_entry(
        &mut entry,
        target,
        value,
        ArtifactType::WorkItem,
        allow_forced_simple_set,
        id,
    )?;
    if is_work_dependency_target(target) {
        validate_work_dependency_edit(config, &entry)?;
    }
    WorkTomlAdapter::write(config, &entry, op)?;
    Ok(())
}

pub(super) fn remove_toml_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    let mut doc = serde_json::to_value(entry.spec())?;
    let (display_field, removed) = remove_target_from_doc(artifact, &mut doc, id, target, opts)?;

    *entry.spec_mut() = serde_json::from_value(doc)?;
    A::write(config, &entry, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}

pub(super) fn tick_toml_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
    status_str: &str,
) -> anyhow::Result<String>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    let mut doc = serde_json::to_value(entry.spec())?;
    let ticked_text = tick_target_in_doc(artifact, &mut doc, id, target, opts, status_str)?;
    *entry.spec_mut() = serde_json::from_value(doc)?;
    A::write(config, &entry, op)?;
    Ok(ticked_text)
}

pub(super) fn is_work_dependency_target(target: &edit_engine::ResolvedTarget) -> bool {
    match target {
        edit_engine::ResolvedTarget::Node { path, .. } => is_work_dependency_path(path),
        edit_engine::ResolvedTarget::IndexedItem { container_path, .. } => {
            is_work_dependency_path(container_path)
        }
    }
}

pub(super) fn validate_work_dependency_edit(
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

fn apply_toml_target_to_entry<E>(
    entry: &mut E,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    artifact: ArtifactType,
    allow_forced_simple_set: bool,
    id: &str,
) -> anyhow::Result<()>
where
    E: TomlEditableEntry,
{
    let mut doc = serde_json::to_value(entry.spec())?;
    match target {
        edit_engine::ResolvedTarget::Node {
            path,
            kind: edit_engine::TargetKind::Scalar,
            origin,
            ..
        } => match origin {
            edit_engine::TargetOrigin::Simple => {
                let simple = path.as_simple().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::E0901IoError,
                        "simple target path expected",
                        id,
                    )
                })?;
                if allow_forced_simple_set {
                    edit_runtime::set_simple_field_forced(artifact, &mut doc, simple, value, id)?;
                } else {
                    edit_runtime::set_simple_field(artifact, &mut doc, simple, value, id)?;
                }
            }
            edit_engine::TargetOrigin::Nested => {
                edit_runtime::set_nested_field(artifact, &mut doc, path, value, id)?;
            }
        },
        edit_engine::ResolvedTarget::IndexedItem {
            origin,
            container_path,
            index,
            item_kind: edit_engine::TargetKind::Scalar,
            ..
        } => match origin {
            edit_engine::TargetOrigin::Simple => {
                let simple = container_path.as_simple().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::E0901IoError,
                        "simple indexed container expected",
                        id,
                    )
                })?;
                edit_runtime::set_simple_list_item(artifact, &mut doc, simple, *index, value, id)?;
            }
            edit_engine::TargetOrigin::Nested => {
                edit_runtime::set_nested_list_item(
                    artifact,
                    &mut doc,
                    container_path,
                    *index,
                    value,
                    id,
                )?;
            }
        },
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "set requires a scalar field or indexed scalar list item",
                id,
            )
            .into());
        }
    }
    *entry.spec_mut() = serde_json::from_value(doc)?;
    Ok(())
}

fn is_work_dependency_path(path: &FieldPath) -> bool {
    path.as_simple() == Some("depends_on") || path.to_string() == "govctl.depends_on"
}

fn render_resolved_target(
    artifact: ArtifactType,
    doc: &serde_json::Value,
    target: &edit_engine::ResolvedTarget,
    id: &str,
) -> anyhow::Result<String> {
    match target {
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Simple,
            path,
            ..
        } => {
            let simple = path.as_simple().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    "simple node target should use a simple path",
                    id,
                )
            })?;
            edit_runtime::get_simple_field(artifact, doc, simple, id)
        }
        edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Simple,
            container_path,
            index,
            ..
        } => {
            let simple = container_path.as_simple().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    "simple indexed target should use a simple container path",
                    id,
                )
            })?;
            edit_runtime::get_simple_list_item(artifact, doc, simple, *index, id)
        }
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Nested,
            path,
            ..
        }
        | edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Nested,
            path,
            ..
        } => edit_runtime::get_nested_field(artifact, doc, path, id),
    }
}

fn tick_target_in_doc(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    status_str: &str,
) -> anyhow::Result<String> {
    match target {
        edit_engine::ResolvedTarget::Node {
            path,
            kind: edit_engine::TargetKind::List,
            origin,
            status_list,
        } => {
            if !status_list {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    TICK_NESTED_PATH_ERROR,
                    id,
                )
                .into());
            }
            match origin {
                edit_engine::TargetOrigin::Simple => {
                    let simple = path.as_simple().ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0901IoError,
                            "simple list target expected",
                            id,
                        )
                    })?;
                    edit_runtime::tick_simple_status_list_item_with_matcher(
                        artifact,
                        doc,
                        simple,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(id, simple, items, opts, MatchUse::TickSingle)
                        },
                    )?
                    .ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown field for tick: {simple}"),
                            id,
                        )
                        .into()
                    })
                }
                edit_engine::TargetOrigin::Nested => {
                    let display = path.to_string();
                    edit_runtime::tick_nested_list_item_with_matcher(
                        artifact,
                        doc,
                        path,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(id, &display, items, opts, MatchUse::TickSingle)
                        },
                    )
                }
            }
        }
        edit_engine::ResolvedTarget::IndexedItem {
            container_path,
            index,
            origin,
            status_list,
            ..
        } => {
            if !status_list {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    TICK_NESTED_PATH_ERROR,
                    id,
                )
                .into());
            }
            let exact = MatchOptions {
                pattern: None,
                at: Some(*index),
                exact: false,
                regex: false,
                all: false,
            };
            match origin {
                edit_engine::TargetOrigin::Simple => {
                    let simple = container_path.as_simple().ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0901IoError,
                            "simple indexed container expected",
                            id,
                        )
                    })?;
                    edit_runtime::tick_simple_status_list_item_with_matcher(
                        artifact,
                        doc,
                        simple,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(id, simple, items, &exact, MatchUse::TickSingle)
                        },
                    )?
                    .ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown field for tick: {simple}"),
                            id,
                        )
                        .into()
                    })
                }
                edit_engine::TargetOrigin::Nested => {
                    edit_runtime::tick_nested_list_item_with_matcher(
                        artifact,
                        doc,
                        container_path,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(
                                id,
                                &container_path.to_string(),
                                items,
                                &exact,
                                MatchUse::TickSingle,
                            )
                        },
                    )
                }
            }
        }
        _ => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            TICK_NESTED_PATH_ERROR,
            id,
        )
        .into()),
    }
}
