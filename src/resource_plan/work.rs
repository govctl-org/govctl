use super::{
    ToPlan, compile_common_add, compile_common_delete, compile_common_edit, compile_common_get,
    compile_common_list, compile_common_remove, compile_common_render, compile_common_set,
    compile_common_show, compile_common_tick,
};
use crate::cmd;
use crate::command_router::{
    CommandPlan, CreateOp, EditExtras, LifecycleOp, plan_create, plan_lifecycle,
};
use crate::{ListTarget, WorkAddArgs, WorkCommand, WorkEditArgs, WorkTickArgs};

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
