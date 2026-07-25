use super::{
    ToPlan, compile_common_delete, compile_common_deprecate, compile_common_edit,
    compile_common_get, compile_common_list, compile_common_show, compile_common_supersede,
};
use crate::cmd;
use crate::command_router::plan_create;
use crate::diagnostic::DiagnosticResult;
use crate::{ClauseCommand, ListTarget};

impl ToPlan for ClauseCommand {
    fn to_plan(&self) -> DiagnosticResult<crate::command_router::CommandPlan> {
        match self {
            ClauseCommand::List(args) => Ok(compile_common_list(ListTarget::Clause, args)),
            ClauseCommand::Get(args) => compile_common_get(args),
            ClauseCommand::Show(args) => {
                Ok(compile_common_show(cmd::edit::ArtifactType::Clause, args))
            }
            ClauseCommand::New {
                clause_id,
                title,
                section,
                kind,
            } => Ok(plan_create(
                ListTarget::Clause,
                crate::command_router::CreateOp::Clause {
                    clause_id: clause_id.clone(),
                    title: title.clone(),
                    section: section.clone(),
                    kind: *kind,
                },
            )),
            ClauseCommand::Edit(args) => compile_common_edit(args),
            ClauseCommand::Delete(args) => {
                compile_common_delete(cmd::edit::ArtifactType::Clause, args)
            }
            ClauseCommand::Deprecate(args) => {
                compile_common_deprecate(cmd::edit::ArtifactType::Clause, args)
            }
            ClauseCommand::Supersede(args) => {
                compile_common_supersede(cmd::edit::ArtifactType::Clause, args)
            }
        }
    }
}
