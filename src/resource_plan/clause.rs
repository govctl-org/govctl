use super::{
    ToPlan, compile_common_delete, compile_common_deprecate, compile_common_get,
    compile_common_list, compile_common_set, compile_common_show, compile_common_supersede,
};
use crate::cmd;
use crate::command_router::{
    EditExtras, EditOp, Op, artifact, owned_edit_action, plan_create, plan_edit,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::{ClauseCommand, EditActionArgs, ListTarget};

impl ToPlan for ClauseCommand {
    fn to_plan(&self) -> anyhow::Result<crate::command_router::CommandPlan> {
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
            ClauseCommand::Edit {
                id,
                path,
                set,
                add,
                remove,
                tick,
                stdin,
                at,
                exact,
                regex,
                all,
                text,
                text_file,
            } => {
                let uses_canonical = path.is_some()
                    || set.is_some()
                    || add.is_some()
                    || remove.is_some()
                    || tick.is_some();
                let uses_legacy = text.is_some() || text_file.is_some();
                if uses_canonical && uses_legacy {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0802ConflictingArgs,
                        "Cannot mix canonical clause edit flags with legacy clause edit flags",
                        id,
                    )
                    .into());
                }
                if !uses_canonical && (at.is_some() || *exact || *regex || *all || path.is_some()) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0802ConflictingArgs,
                        "Legacy clause edit does not support canonical path or matcher flags",
                        id,
                    )
                    .into());
                }
                if uses_canonical {
                    let path = path.clone().ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0801MissingRequiredArg,
                            "canonical clause edit requires a field path before --set/--add/--remove/--tick",
                            id,
                        )
                    })?;
                    plan_edit(
                        id,
                        &path,
                        owned_edit_action(&EditActionArgs {
                            set: set.clone(),
                            add: add.clone(),
                            remove: remove.clone(),
                            tick: *tick,
                            stdin: *stdin,
                            at: *at,
                            exact: *exact,
                            regex: *regex,
                            all: *all,
                        })?,
                        EditExtras::default(),
                    )
                } else {
                    Ok(artifact(
                        cmd::edit::ArtifactType::Clause,
                        id,
                        Op::Edit(EditOp::ClauseLegacy {
                            text: text.clone(),
                            text_file: text_file.clone(),
                            stdin: *stdin,
                        }),
                    ))
                }
            }
            ClauseCommand::Set(args) => compile_common_set(args),
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
