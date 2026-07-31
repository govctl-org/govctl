use crate::Cli;
use clap::{Command, CommandFactory};
use serde::Serialize;

#[derive(Serialize)]
pub struct CommandInfo {
    pub name: String,
    pub summary: String,
    pub usage: String,
    pub subcommands: Vec<CommandInfo>,
}

#[derive(Serialize)]
pub struct WorkflowInfo {
    pub phase_order: [&'static str; 4],
}

pub(super) fn command_catalog() -> Vec<CommandInfo> {
    let root = Cli::command();
    let root_name = root.get_name().to_string();

    let mut commands: Vec<_> = root
        .get_subcommands()
        .filter(|command| !command.is_hide_set())
        .map(|command| command_info(command, &root_name))
        .collect();
    commands.sort_by(|left, right| left.name.cmp(&right.name));
    commands
}

fn command_info(command: &Command, parent_path: &str) -> CommandInfo {
    let name = command.get_name().to_string();
    let command_path = format!("{parent_path} {name}");
    let mut rendered = command.clone().bin_name(&command_path);
    let usage = rendered
        .render_usage()
        .to_string()
        .trim()
        .strip_prefix("Usage: ")
        .unwrap_or_default()
        .to_string();
    let mut subcommands: Vec<_> = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .map(|subcommand| command_info(subcommand, &command_path))
        .collect();
    subcommands.sort_by(|left, right| left.name.cmp(&right.name));

    CommandInfo {
        name,
        summary: command
            .get_about()
            .map(ToString::to_string)
            .unwrap_or_default(),
        usage,
        subcommands,
    }
}

pub(super) fn workflow_info() -> WorkflowInfo {
    WorkflowInfo {
        phase_order: ["spec", "impl", "test", "stable"],
    }
}
