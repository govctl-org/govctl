use super::{BuiltinOp, CommandPlan, Op, global};
use crate::Commands;
use crate::diagnostic::DiagnosticResult;

impl CommandPlan {
    pub fn from_parsed(cmd: &Commands, global_dry_run: bool) -> DiagnosticResult<Self> {
        use crate::resource_plan::ToPlan;

        match cmd {
            Commands::Init { force } => Ok(global(Op::Builtin(BuiltinOp::Init { force: *force }))),
            Commands::InitSkills { force, format, dir } => {
                Ok(global(Op::Builtin(BuiltinOp::InitSkills {
                    force: *force,
                    format: format.clone(),
                    dir: dir.clone(),
                })))
            }
            Commands::Check { has_active, .. } => Ok(global(Op::Builtin(BuiltinOp::Check {
                has_active: *has_active,
            }))),
            Commands::Status => Ok(global(Op::Builtin(BuiltinOp::Status))),
            Commands::Render {
                target,
                dry_run,
                force,
            } => Ok(global(Op::Builtin(BuiltinOp::RenderGlobal {
                target: *target,
                dry_run: global_dry_run || *dry_run,
                force: *force,
            }))),
            Commands::Migrate => Ok(global(Op::Builtin(BuiltinOp::Migrate))),
            Commands::Verify { guard_ids, work } => Ok(global(Op::Builtin(BuiltinOp::Verify {
                guard_ids: guard_ids.clone(),
                work: work.clone(),
            }))),
            Commands::Describe { context, .. } => Ok(global(Op::Builtin(BuiltinOp::Describe {
                context: *context,
            }))),
            Commands::Completions { shell } => Ok(global(Op::Builtin(BuiltinOp::Completions {
                shell: *shell,
            }))),
            Commands::SelfUpdate { check } => {
                Ok(global(Op::Builtin(BuiltinOp::SelfUpdate { check: *check })))
            }
            #[cfg(feature = "tui")]
            Commands::Tui => Ok(global(Op::Builtin(BuiltinOp::Tui))),
            Commands::Rfc { command } => command.to_plan(),
            Commands::Clause { command } => command.to_plan(),
            Commands::Adr { command } => command.to_plan(),
            Commands::Work { command } => command.to_plan(),
            Commands::Guard { command } => command.to_plan(),
            Commands::Loop { command } => match command {
                crate::LoopCommand::List {
                    filter,
                    limit,
                    output,
                } => Ok(global(Op::Builtin(BuiltinOp::LoopList {
                    filter: filter.clone(),
                    limit: *limit,
                    output: *output,
                }))),
                crate::LoopCommand::Start { id, work_items } => {
                    Ok(global(Op::Builtin(BuiltinOp::LoopStart {
                        loop_id: id.clone(),
                        work_items: work_items.clone(),
                    })))
                }
                crate::LoopCommand::Show { id } => Ok(global(Op::Builtin(BuiltinOp::LoopShow {
                    loop_id: id.clone(),
                }))),
                crate::LoopCommand::Resume { id } => {
                    Ok(global(Op::Builtin(BuiltinOp::LoopResume {
                        loop_id: id.clone(),
                    })))
                }
                crate::LoopCommand::Replan { id } => {
                    Ok(global(Op::Builtin(BuiltinOp::LoopReplan {
                        loop_id: id.clone(),
                    })))
                }
                crate::LoopCommand::Add { id, field, value } => {
                    Ok(global(Op::Builtin(BuiltinOp::LoopAdd {
                        loop_id: id.clone(),
                        field: field.clone(),
                        value: value.clone(),
                    })))
                }
                crate::LoopCommand::Remove { id, field, value } => {
                    Ok(global(Op::Builtin(BuiltinOp::LoopRemove {
                        loop_id: id.clone(),
                        field: field.clone(),
                        value: value.clone(),
                    })))
                }
                crate::LoopCommand::Run {
                    id,
                    target_work_items,
                    max_rounds,
                } => Ok(global(Op::Builtin(BuiltinOp::LoopRun {
                    loop_id: id.clone(),
                    target_work_items: target_work_items.clone(),
                    max_rounds: *max_rounds,
                }))),
            },
            Commands::Release { version, date } => Ok(global(Op::Builtin(BuiltinOp::ReleaseCut {
                version: version.clone(),
                date: date.clone(),
            }))),
            Commands::Tag { command } => match command {
                crate::TagCommand::New { tag } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagNew { tag: tag.clone() })))
                }
                crate::TagCommand::Delete { tag } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagDelete {
                        tag: tag.clone(),
                    })))
                }
                crate::TagCommand::List { output } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagList { output: *output })))
                }
            },
        }
    }
}
