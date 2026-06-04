use crate::cmd;
use crate::command_router::{
    CommandPlan, EditExtras, LifecycleOp, OwnedMatchOptions, add_action, owned_edit_action,
    plan_artifact_render, plan_delete, plan_edit, plan_get, plan_lifecycle, plan_list, plan_show,
    remove_action, set_action, tick_action,
};
use crate::diagnostic::DiagnosticResult;
use crate::{
    CommonAddArgs, CommonDeleteArgs, CommonDeprecateArgs, CommonEditArgs, CommonGetArgs,
    CommonListArgs, CommonRemoveArgs, CommonRenderArgs, CommonSetArgs, CommonShowArgs,
    CommonSupersedeArgs, CommonTickSelectorArgs, ListTarget, TickStatus,
};

mod adr;
mod clause;
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
    plan_get(&args.id, args.field.as_deref())
}

fn compile_common_show(artifact: cmd::edit::ArtifactType, args: &CommonShowArgs) -> CommandPlan {
    plan_show(artifact, &args.id, args.output)
}

fn compile_common_edit(args: &CommonEditArgs, extras: EditExtras) -> DiagnosticResult<CommandPlan> {
    plan_edit(
        &args.id,
        &args.path,
        owned_edit_action(&args.action)?,
        extras,
    )
}

fn compile_common_set(args: &CommonSetArgs) -> DiagnosticResult<CommandPlan> {
    plan_edit(
        &args.id,
        &args.field,
        set_action(args.value.clone(), args.stdin),
        EditExtras::default(),
    )
}

fn compile_common_add(args: &CommonAddArgs, extras: EditExtras) -> DiagnosticResult<CommandPlan> {
    plan_edit(
        &args.id,
        &args.field,
        add_action(args.value.clone(), args.stdin),
        extras,
    )
}

fn compile_common_remove(args: &CommonRemoveArgs) -> DiagnosticResult<CommandPlan> {
    plan_edit(
        &args.id,
        &args.field,
        remove_action(OwnedMatchOptions {
            pattern: args.pattern.clone(),
            at: args.at,
            exact: args.exact,
            regex: args.regex,
            all: args.all,
        }),
        EditExtras::default(),
    )
}

fn compile_common_tick(
    args: &CommonTickSelectorArgs,
    status: TickStatus,
) -> DiagnosticResult<CommandPlan> {
    plan_edit(
        &args.id,
        &args.field,
        tick_action(
            OwnedMatchOptions {
                pattern: args.pattern.clone(),
                at: args.at,
                exact: args.exact,
                regex: args.regex,
                all: false,
            },
            status,
        ),
        EditExtras::default(),
    )
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
