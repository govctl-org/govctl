use crate::cmd;
use crate::command_router::{
    CommandPlan, CreateOp, EditExtras, EditOp, LifecycleOp, Op, OwnedMatchOptions, add_action,
    artifact, owned_edit_action, plan_artifact_render, plan_create, plan_delete, plan_edit,
    plan_get, plan_lifecycle, plan_list, plan_show, remove_action, set_action, tick_action,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::write::BumpLevel;
use crate::{
    AdrAddArgs, AdrCommand, AdrEditArgs, AdrTickArgs, ClauseCommand, CommonAddArgs,
    CommonDeleteArgs, CommonDeprecateArgs, CommonEditArgs, CommonGetArgs, CommonIdArgs,
    CommonListArgs, CommonRemoveArgs, CommonRenderArgs, CommonSetArgs, CommonShowArgs,
    CommonSupersedeArgs, CommonTickSelectorArgs, EditActionArgs, GuardAddArgs, GuardCommand,
    ListTarget, RfcCommand, TickStatus, WorkAddArgs, WorkCommand, WorkEditArgs, WorkTickArgs,
};

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

impl ToPlan for ClauseCommand {
    fn to_plan(&self) -> anyhow::Result<CommandPlan> {
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
                CreateOp::Clause {
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
