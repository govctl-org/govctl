use super::{
    ToPlan, compile_common_delete, compile_common_edit, compile_common_get, compile_common_list,
    compile_common_show,
};
use crate::cmd;
use crate::command_router::{CommandPlan, CreateOp, plan_create};
use crate::diagnostic::DiagnosticResult;
use crate::{GuardCommand, ListTarget};

impl ToPlan for GuardCommand {
    fn to_plan(&self) -> DiagnosticResult<CommandPlan> {
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
            GuardCommand::Edit(args) => compile_common_edit(args),
            GuardCommand::Delete(args) => {
                compile_common_delete(cmd::edit::ArtifactType::Guard, args)
            }
        }
    }
}
