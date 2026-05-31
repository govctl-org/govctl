use super::{
    ToPlan, compile_common_add, compile_common_deprecate, compile_common_edit, compile_common_get,
    compile_common_list, compile_common_remove, compile_common_render, compile_common_set,
    compile_common_show, compile_common_supersede,
};
use crate::cmd;
use crate::command_router::{
    CommandPlan, CreateOp, EditExtras, LifecycleOp, plan_create, plan_lifecycle,
};
use crate::write::BumpLevel;
use crate::{ListTarget, RfcCommand};

impl ToPlan for RfcCommand {
    fn to_plan(&self) -> anyhow::Result<CommandPlan> {
        match self {
            RfcCommand::List(args) => Ok(compile_common_list(ListTarget::Rfc, args)),
            RfcCommand::Get(args) => compile_common_get(args),
            RfcCommand::Show(args) => Ok(compile_common_show(cmd::edit::ArtifactType::Rfc, args)),
            RfcCommand::New { title, id } => Ok(plan_create(
                ListTarget::Rfc,
                CreateOp::Rfc {
                    title: title.clone(),
                    id: id.clone(),
                },
            )),
            RfcCommand::Edit(args) => compile_common_edit(args, EditExtras::default()),
            RfcCommand::Set(args) => compile_common_set(args),
            RfcCommand::Add(args) => compile_common_add(args, EditExtras::default()),
            RfcCommand::Remove(args) => compile_common_remove(args),
            RfcCommand::Bump {
                id,
                patch,
                minor,
                major,
                summary,
                changes,
            } => {
                let level = match (patch, minor, major) {
                    (true, false, false) => Some(BumpLevel::Patch),
                    (false, true, false) => Some(BumpLevel::Minor),
                    (false, false, true) => Some(BumpLevel::Major),
                    (false, false, false) => None,
                    _ => unreachable!("clap arg group ensures mutual exclusivity"),
                };
                Ok(plan_lifecycle(
                    cmd::edit::ArtifactType::Rfc,
                    id,
                    LifecycleOp::Bump {
                        level,
                        summary: summary.clone(),
                        changes: changes.clone(),
                    },
                ))
            }
            RfcCommand::Finalize { id, status } => Ok(plan_lifecycle(
                cmd::edit::ArtifactType::Rfc,
                id,
                LifecycleOp::Finalize { status: *status },
            )),
            RfcCommand::Advance { id, phase } => Ok(plan_lifecycle(
                cmd::edit::ArtifactType::Rfc,
                id,
                LifecycleOp::Advance { phase: *phase },
            )),
            RfcCommand::Deprecate(args) => {
                compile_common_deprecate(cmd::edit::ArtifactType::Rfc, args)
            }
            RfcCommand::Supersede(args) => {
                compile_common_supersede(cmd::edit::ArtifactType::Rfc, args)
            }
            RfcCommand::Render(args) => compile_common_render(cmd::edit::ArtifactType::Rfc, args),
        }
    }
}
