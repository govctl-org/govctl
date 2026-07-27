use super::{
    ToPlan, compile_common_delete, compile_common_edit, compile_common_get, compile_common_list,
    compile_common_render, compile_common_show,
};
use crate::cmd;
use crate::command_router::{CommandPlan, CreateOp, LifecycleOp, plan_create, plan_lifecycle};
use crate::diagnostic::DiagnosticResult;
use crate::{ListTarget, WorkCommand};

impl ToPlan for WorkCommand {
    fn to_plan(&self) -> DiagnosticResult<CommandPlan> {
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
            WorkCommand::Edit(args) => compile_common_edit(args),
            WorkCommand::Delete(args) => {
                compile_common_delete(cmd::edit::ArtifactType::WorkItem, args)
            }
            WorkCommand::Render(args) => {
                compile_common_render(cmd::edit::ArtifactType::WorkItem, args)
            }
        }
    }
}
