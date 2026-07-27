use super::ToPlan;
use crate::ConformanceCommand;
use crate::command_router::{BuiltinOp, CommandPlan, Op, owned_edit_action};
use crate::diagnostic::DiagnosticResult;

impl ToPlan for ConformanceCommand {
    fn to_plan(&self) -> DiagnosticResult<CommandPlan> {
        let op = match self {
            ConformanceCommand::List(args) => BuiltinOp::ConformanceList {
                filter: args.filter.clone(),
                limit: args.limit,
                output: args.output,
                tags: args
                    .tag
                    .as_deref()
                    .map(|tags| {
                        tags.split(',')
                            .map(str::trim)
                            .filter(|tag| !tag.is_empty())
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            ConformanceCommand::Get(args) => BuiltinOp::ConformanceGet {
                id: args.id.clone(),
                field: args.field.clone(),
                output: args.output,
            },
            ConformanceCommand::Show(args) => BuiltinOp::ConformanceShow {
                id: args.id.clone(),
                output: args.output,
                history: args.history,
            },
            ConformanceCommand::New {
                title,
                path,
                selector,
                requirements,
                guards,
                id,
            } => BuiltinOp::ConformanceNew {
                title: title.clone(),
                path: path.clone(),
                selector: selector.clone(),
                requirements: requirements.clone(),
                guards: guards.clone(),
                id: id.clone(),
            },
            ConformanceCommand::Edit(args) => BuiltinOp::ConformanceEdit {
                id: args.id.clone(),
                path: args.path.clone(),
                action: owned_edit_action(&args.action)?,
            },
            ConformanceCommand::Delete(args) => BuiltinOp::ConformanceDelete {
                id: args.id.clone(),
                force: args.force,
            },
            ConformanceCommand::Trace { target, output } => BuiltinOp::ConformanceTrace {
                target: target.clone(),
                output: *output,
            },
        };
        Ok(CommandPlan::new(
            crate::command_router::Scope::Global,
            Op::Builtin(op),
        ))
    }
}
