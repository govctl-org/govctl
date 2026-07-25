use super::{
    ToPlan, compile_common_deprecate, compile_common_edit, compile_common_get, compile_common_list,
    compile_common_render, compile_common_show, compile_common_supersede,
};
use crate::cmd;
use crate::command_router::{CommandPlan, CreateOp, LifecycleOp, plan_create, plan_lifecycle};
use crate::diagnostic::DiagnosticResult;
use crate::{AdrCommand, CommonIdArgs, ListTarget};

impl ToPlan for AdrCommand {
    fn to_plan(&self) -> DiagnosticResult<CommandPlan> {
        match self {
            AdrCommand::List(args) => Ok(compile_common_list(ListTarget::Adr, args)),
            AdrCommand::Get(args) => compile_common_get(args),
            AdrCommand::Show(args) => Ok(compile_common_show(cmd::edit::ArtifactType::Adr, args)),
            AdrCommand::New { title } => Ok(plan_create(
                ListTarget::Adr,
                CreateOp::Adr {
                    title: title.clone(),
                },
            )),
            AdrCommand::Edit(args) => compile_common_edit(args),
            AdrCommand::Accept { id, force } => Ok(plan_lifecycle(
                cmd::edit::ArtifactType::Adr,
                id,
                LifecycleOp::AcceptAdr { force: *force },
            )),
            AdrCommand::Reject(CommonIdArgs { id }) => Ok(plan_lifecycle(
                cmd::edit::ArtifactType::Adr,
                id,
                LifecycleOp::RejectAdr,
            )),
            AdrCommand::Deprecate(args) => {
                compile_common_deprecate(cmd::edit::ArtifactType::Adr, args)
            }
            AdrCommand::Supersede(args) => {
                compile_common_supersede(cmd::edit::ArtifactType::Adr, args)
            }
            AdrCommand::Render(args) => compile_common_render(cmd::edit::ArtifactType::Adr, args),
        }
    }
}
