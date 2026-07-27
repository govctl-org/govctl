use crate::cmd;
use crate::command_router::{
    CommandPlan, LifecycleOp, owned_edit_action, plan_artifact_render, plan_delete, plan_edit,
    plan_get, plan_lifecycle, plan_list, plan_show,
};
use crate::diagnostic::DiagnosticResult;
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

fn compile_common_list(target: ListTarget, args: &CommonListArgs) -> CommandPlan {
    // Parse comma-separated tags from --tag option
    let tags: Vec<String> = args
        .tag
        .as_deref()
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    plan_list(target, args.filter.clone(), args.limit, args.output, tags)
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
