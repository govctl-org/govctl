use super::{BuiltinOp, CommandPlan, Op, global};
use crate::diagnostic::DiagnosticResult;
use crate::{Commands, LoopCommand, TagCommand};

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
            Commands::Loop { command } => Ok(plan_loop_command(command)),
            Commands::Release { version, date } => Ok(global(Op::Builtin(BuiltinOp::ReleaseCut {
                version: version.clone(),
                date: date.clone(),
            }))),
            Commands::Tag { command } => Ok(plan_tag_command(command)),
        }
    }
}

fn plan_loop_command(command: &LoopCommand) -> CommandPlan {
    let op = match command {
        LoopCommand::List {
            filter,
            limit,
            output,
        } => BuiltinOp::LoopList {
            filter: filter.clone(),
            limit: *limit,
            output: *output,
        },
        LoopCommand::Start { id, work_ids } => BuiltinOp::LoopStart {
            loop_id: id.clone(),
            work_ids: work_ids.clone(),
        },
        LoopCommand::Show { id } => BuiltinOp::LoopShow {
            loop_id: id.clone(),
        },
        LoopCommand::Resume { id } => BuiltinOp::LoopResume {
            loop_id: id.clone(),
        },
        LoopCommand::Replan { id } => BuiltinOp::LoopReplan {
            loop_id: id.clone(),
        },
        LoopCommand::Add { id, field, value } => BuiltinOp::LoopAdd {
            loop_id: id.clone(),
            field: field.clone(),
            value: value.clone(),
        },
        LoopCommand::Remove { id, field, value } => BuiltinOp::LoopRemove {
            loop_id: id.clone(),
            field: field.clone(),
            value: value.clone(),
        },
        LoopCommand::Run {
            id,
            target_work_ids,
            max_rounds,
        } => BuiltinOp::LoopRun {
            loop_id: id.clone(),
            target_work_ids: target_work_ids.clone(),
            max_rounds: *max_rounds,
        },
    };
    global(Op::Builtin(op))
}

fn plan_tag_command(command: &TagCommand) -> CommandPlan {
    let op = match command {
        TagCommand::New { tag } => BuiltinOp::TagNew { tag: tag.clone() },
        TagCommand::Delete { tag } => BuiltinOp::TagDelete { tag: tag.clone() },
        TagCommand::List { output } => BuiltinOp::TagList { output: *output },
    };
    global(Op::Builtin(op))
}
