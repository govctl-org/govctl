use super::{
    ToPlan, compile_common_add, compile_common_deprecate, compile_common_edit, compile_common_get,
    compile_common_list, compile_common_remove, compile_common_render, compile_common_set,
    compile_common_show, compile_common_supersede, compile_common_tick,
};
use crate::cmd;
use crate::command_router::{
    CommandPlan, CreateOp, EditExtras, LifecycleOp, plan_create, plan_lifecycle,
};
use crate::{AdrAddArgs, AdrCommand, AdrEditArgs, AdrTickArgs, CommonIdArgs, ListTarget};

impl ToPlan for AdrCommand {
    fn to_plan(&self) -> anyhow::Result<CommandPlan> {
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
            AdrCommand::Edit(AdrEditArgs {
                common,
                pro,
                con,
                reject_reason,
            }) => compile_common_edit(
                common,
                EditExtras {
                    pros: pro.clone(),
                    cons: con.clone(),
                    reject_reason: reject_reason.clone(),
                    ..EditExtras::default()
                },
            ),
            AdrCommand::Set(args) => compile_common_set(args),
            AdrCommand::Add(AdrAddArgs {
                common,
                pro,
                con,
                reject_reason,
            }) => compile_common_add(
                common,
                EditExtras {
                    pros: pro.clone(),
                    cons: con.clone(),
                    reject_reason: reject_reason.clone(),
                    ..EditExtras::default()
                },
            ),
            AdrCommand::Remove(args) => compile_common_remove(args),
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
            AdrCommand::Tick(AdrTickArgs { common, status }) => {
                compile_common_tick(common, (*status).into())
            }
            AdrCommand::Render(args) => compile_common_render(cmd::edit::ArtifactType::Adr, args),
        }
    }
}
