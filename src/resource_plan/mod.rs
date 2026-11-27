use crate::cmd;
use crate::command_router::{
    CommandPlan, LifecycleOp, owned_edit_action, plan_artifact_render, plan_delete, plan_edit,
    plan_get, plan_lifecycle, plan_list, plan_show,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::{
    CommonDeleteArgs, CommonDeprecateArgs, CommonEditArgs, CommonGetArgs, CommonListArgs,
    CommonRenderArgs, CommonShowArgs, CommonSupersedeArgs, ListTarget,
};

mod adr;
mod clause;
mod conformance;
mod guard;
mod rfc;
mod work;

pub(crate) trait ToPlan {
    fn to_plan(&self) -> DiagnosticResult<CommandPlan>;
}

fn compile_tags(tag: Option<&str>) -> Vec<String> {
    tag.map(|value| {
        value
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

fn compile_common_list(target: ListTarget, args: &CommonListArgs) -> CommandPlan {
    plan_list(
        target,
        args.filter.clone(),
        args.limit,
        args.output,
        compile_tags(args.tag.as_deref()),
        Vec::new(),
    )
}

/// Work Item list. `--owner` is trimmed and a whitespace-only value is empty.
/// [[RFC-0002:C-WORK-OWNERS]]
fn compile_work_list(args: &CommonListArgs, owners: &[String]) -> DiagnosticResult<CommandPlan> {
    let mut normalized = Vec::with_capacity(owners.len());
    for owner in owners {
        let trimmed = owner.trim();
        if trimmed.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0805EmptyValue,
                "Owner filter must not be empty",
                "work list",
            ));
        }
        normalized.push(trimmed.to_string());
    }
    Ok(plan_list(
        ListTarget::Work,
        args.filter.clone(),
        args.limit,
        args.output,
        compile_tags(args.tag.as_deref()),
        normalized,
    ))
}

fn compile_common_get(args: &CommonGetArgs) -> DiagnosticResult<CommandPlan> {
    plan_get(&args.id, args.field.as_deref(), args.output)
}

fn compile_common_show(artifact: cmd::edit::ArtifactType, args: &CommonShowArgs) -> CommandPlan {
    plan_show(artifact, &args.id, args.output, args.history)
}

fn compile_common_edit(args: &CommonEditArgs) -> DiagnosticResult<CommandPlan> {
    plan_edit(&args.id, &args.path, owned_edit_action(&args.action)?)
}

fn compile_common_render(
    artifact: cmd::edit::ArtifactType,
    args: &CommonRenderArgs,
) -> DiagnosticResult<CommandPlan> {
    Ok(plan_artifact_render(artifact, &args.id, args.dry_run))
}

fn compile_common_delete(
    artifact: cmd::edit::ArtifactType,
    args: &CommonDeleteArgs,
) -> DiagnosticResult<CommandPlan> {
    Ok(plan_delete(artifact, &args.id, args.force))
}

fn compile_common_deprecate(
    artifact: cmd::edit::ArtifactType,
    args: &CommonDeprecateArgs,
) -> DiagnosticResult<CommandPlan> {
    Ok(plan_lifecycle(
        artifact,
        &args.id,
        LifecycleOp::Deprecate { force: args.force },
    ))
}

fn compile_common_supersede(
    artifact: cmd::edit::ArtifactType,
    args: &CommonSupersedeArgs,
) -> DiagnosticResult<CommandPlan> {
    Ok(plan_lifecycle(
        artifact,
        &args.id,
        LifecycleOp::Supersede {
            by: args.by.clone(),
            force: args.force,
        },
    ))
}
