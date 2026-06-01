//! Command planning for unified routing semantics.
//!
//! This module compiles parsed CLI syntax into semantic execution plans built
//! from `Scope + Op`. The planner is the single normalization point for both
//! canonical and compatibility command forms.

mod edit_action;
mod execute;
mod parsed;
mod plan;

use crate::cmd;
use crate::diagnostic::DiagnosticResult;
use crate::{ListTarget, OutputFormat};

pub(crate) type OwnedMatchOptions = cmd::edit::MatchOptionsOwned;
pub(crate) type OwnedEditAction = cmd::edit::OwnedEditAction;

pub(crate) use edit_action::{
    add_action, owned_edit_action, remove_action, set_action, tick_action,
};
pub use plan::{
    BuiltinOp, CommandPlan, CreateOp, EditExtras, EditOp, LifecycleOp, LockDisposition, Op, Scope,
};

fn artifact_scope(artifact: cmd::edit::ArtifactType, id: &str) -> Scope {
    Scope::Artifact {
        artifact,
        id: id.to_string(),
    }
}

fn resolve_scope(id: &str, field: Option<&str>) -> DiagnosticResult<Scope> {
    let plan = cmd::edit::engine::plan_request(id, field)?;
    Ok(match plan.target {
        Some(target) => Scope::Target {
            artifact: plan.artifact,
            id: id.to_string(),
            target,
        },
        None => artifact_scope(plan.artifact, id),
    })
}

fn global(op: Op) -> CommandPlan {
    CommandPlan::new(Scope::Global, op)
}

fn collection(target: ListTarget, op: Op) -> CommandPlan {
    CommandPlan::new(Scope::Collection { target }, op)
}

pub(crate) fn artifact(artifact: cmd::edit::ArtifactType, id: &str, op: Op) -> CommandPlan {
    CommandPlan::new(artifact_scope(artifact, id), op)
}

fn target(id: &str, field: Option<&str>, op: Op) -> DiagnosticResult<CommandPlan> {
    Ok(CommandPlan::new(resolve_scope(id, field)?, op))
}

fn edit_op_with_extras(action: OwnedEditAction, extras: EditExtras) -> Op {
    Op::Edit(EditOp::Field { action, extras })
}

pub(crate) fn plan_create(collection_target: ListTarget, create: CreateOp) -> CommandPlan {
    collection(collection_target, Op::Create(create))
}

pub(crate) fn plan_list(
    target_kind: ListTarget,
    filter: Option<String>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: Vec<String>,
) -> CommandPlan {
    collection(
        target_kind,
        Op::List {
            filter,
            limit,
            output,
            tags,
        },
    )
}

pub(crate) fn plan_get(id: &str, field: Option<&str>) -> DiagnosticResult<CommandPlan> {
    target(id, field, Op::Get)
}

pub(crate) fn plan_show(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    output: OutputFormat,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Show { output })
}

pub(crate) fn plan_edit(
    id: &str,
    field: &str,
    action: OwnedEditAction,
    extras: EditExtras,
) -> DiagnosticResult<CommandPlan> {
    target(id, Some(field), edit_op_with_extras(action, extras))
}

pub(crate) fn plan_lifecycle(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    lifecycle: LifecycleOp,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Lifecycle(lifecycle))
}

pub(crate) fn plan_artifact_render(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    dry_run: bool,
) -> CommandPlan {
    artifact(artifact_type, id, Op::RenderArtifact { dry_run })
}

pub(crate) fn plan_delete(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    force: bool,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Delete { force })
}

#[cfg(test)]
mod tests;
