use crate::cmd;
use crate::command_router::{
    CommandPlan, CreateOp, EditExtras, LifecycleOp, OwnedMatchOptions, add_action,
    owned_edit_action, plan_artifact_render, plan_create, plan_delete, plan_edit, plan_get,
    plan_lifecycle, plan_list, plan_show, remove_action, set_action, tick_action,
};
use crate::{
    CommonAddArgs, CommonDeleteArgs, CommonDeprecateArgs, CommonEditArgs, CommonGetArgs,
    CommonListArgs, CommonRemoveArgs, CommonRenderArgs, CommonSetArgs, CommonShowArgs,
    CommonSupersedeArgs, CommonTickSelectorArgs, GuardAddArgs, GuardCommand, ListTarget,
    TickStatus, WorkAddArgs, WorkCommand, WorkEditArgs, WorkTickArgs,
};

mod adr;
mod clause;
mod rfc;

pub(crate) trait ToPlan {
    fn to_plan(&self) -> anyhow::Result<CommandPlan>;
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

fn compile_common_get(args: &CommonGetArgs) -> anyhow::Result<CommandPlan> {
    plan_get(&args.id, args.field.as_deref())
}

fn compile_common_show(artifact: cmd::edit::ArtifactType, args: &CommonShowArgs) -> CommandPlan {
    plan_show(artifact, &args.id, args.output)
}

fn compile_common_edit(args: &CommonEditArgs, extras: EditExtras) -> anyhow::Result<CommandPlan> {
    plan_edit(
        &args.id,
        &args.path,
        owned_edit_action(&args.action)?,
        extras,
    )
}

fn compile_common_set(args: &CommonSetArgs) -> anyhow::Result<CommandPlan> {
    plan_edit(
        &args.id,
        &args.field,
        set_action(args.value.clone(), args.stdin),
        EditExtras::default(),
    )
}

fn compile_common_add(args: &CommonAddArgs, extras: EditExtras) -> anyhow::Result<CommandPlan> {
    plan_edit(
        &args.id,
        &args.field,
        add_action(args.value.clone(), args.stdin),
        extras,
    )
}

fn compile_common_remove(args: &CommonRemoveArgs) -> anyhow::Result<CommandPlan> {
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
) -> anyhow::Result<CommandPlan> {
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
) -> anyhow::Result<CommandPlan> {
    Ok(plan_artifact_render(artifact, &args.id, args.dry_run))
}

fn compile_common_delete(
    artifact: cmd::edit::ArtifactType,
    args: &CommonDeleteArgs,
) -> anyhow::Result<CommandPlan> {
    Ok(plan_delete(artifact, &args.id, args.force))
}

fn compile_common_deprecate(
    artifact: cmd::edit::ArtifactType,
    args: &CommonDeprecateArgs,
) -> anyhow::Result<CommandPlan> {
    Ok(plan_lifecycle(
        artifact,
        &args.id,
        LifecycleOp::Deprecate { force: args.force },
    ))
}

fn compile_common_supersede(
    artifact: cmd::edit::ArtifactType,
    args: &CommonSupersedeArgs,
) -> anyhow::Result<CommandPlan> {
    Ok(plan_lifecycle(
        artifact,
        &args.id,
        LifecycleOp::Supersede {
            by: args.by.clone(),
            force: args.force,
        },
    ))
}

impl ToPlan for WorkCommand {
    fn to_plan(&self) -> anyhow::Result<CommandPlan> {
        match self {
            WorkCommand::List(args) => Ok(compile_common_list(ListTarget::Work, args)),
            WorkCommand::Get(args) => compile_common_get(args),
            WorkCommand::Show(args) => {
                Ok(compile_common_show(cmd::edit::ArtifactType::WorkItem, args))
            }
            WorkCommand::Move { file, status } => Ok(plan_lifecycle(
                cmd::edit::ArtifactType::WorkItem,
                &file.display().to_string(),
                LifecycleOp::MoveWork {
                    file_or_id: file.clone(),
                    status: *status,
                },
            )),
            WorkCommand::New { title, active } => Ok(plan_create(
                ListTarget::Work,
                CreateOp::Work {
                    title: title.clone(),
                    active: *active,
                },
            )),
            WorkCommand::Edit(WorkEditArgs {
                common,
                category,
                scope,
            }) => compile_common_edit(
                common,
                EditExtras {
                    category: *category,
                    scope: scope.clone(),
                    ..EditExtras::default()
                },
            ),
            WorkCommand::Set(args) => compile_common_set(args),
            WorkCommand::Add(WorkAddArgs {
                common,
                category,
                scope,
            }) => compile_common_add(
                common,
                EditExtras {
                    category: *category,
                    scope: scope.clone(),
                    ..EditExtras::default()
                },
            ),
            WorkCommand::Remove(args) => compile_common_remove(args),
            WorkCommand::Tick(WorkTickArgs { common, status }) => {
                compile_common_tick(common, (*status).into())
            }
            WorkCommand::Delete(args) => {
                compile_common_delete(cmd::edit::ArtifactType::WorkItem, args)
            }
            WorkCommand::Render(args) => {
                compile_common_render(cmd::edit::ArtifactType::WorkItem, args)
            }
        }
    }
}

impl ToPlan for GuardCommand {
    fn to_plan(&self) -> anyhow::Result<CommandPlan> {
        match self {
            GuardCommand::List(args) => Ok(compile_common_list(ListTarget::Guard, args)),
            GuardCommand::Get(args) => compile_common_get(args),
            GuardCommand::Show(args) => {
                Ok(compile_common_show(cmd::edit::ArtifactType::Guard, args))
            }
            GuardCommand::New { title } => Ok(plan_create(
                ListTarget::Guard,
                CreateOp::Guard {
                    title: title.clone(),
                },
            )),
            GuardCommand::Edit(args) => compile_common_edit(args, EditExtras::default()),
            GuardCommand::Set(args) => compile_common_set(args),
            GuardCommand::Add(GuardAddArgs { id, field, value }) => plan_edit(
                id,
                field,
                add_action(Some(value.clone()), false),
                EditExtras::default(),
            ),
            GuardCommand::Remove(args) => compile_common_remove(args),
            GuardCommand::Delete(args) => {
                compile_common_delete(cmd::edit::ArtifactType::Guard, args)
            }
        }
    }
}
