use super::{
    ToPlan, compile_common_delete, compile_common_edit, compile_common_get, compile_common_list,
    compile_common_remove, compile_common_set, compile_common_show,
};
use crate::cmd;
use crate::command_router::{
    CommandPlan, CreateOp, EditExtras, add_action, plan_create, plan_edit,
};
use crate::{GuardAddArgs, GuardCommand, ListTarget};

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
