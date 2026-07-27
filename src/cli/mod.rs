//! CLI argument definitions for govctl.

mod commands;
mod common;
mod help;
mod loop_cmd;
mod recovery;
mod resources;

pub(crate) use commands::{Commands, ReleaseArgs, ReleaseCommand};
pub(crate) use common::*;
pub(crate) use loop_cmd::LoopCommand;
pub(crate) use recovery::{misrouted_clause_after_parse, misrouted_nested_clause};
pub(crate) use resources::*;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "govctl")]
#[command(about = "Project governance CLI for RFC, ADR, and Work Item management")]
#[command(version)]
pub(crate) struct Cli {
    /// Path to govctl config (TOML)
    #[arg(short = 'C', long, global = true)]
    pub(crate) config: Option<PathBuf>,

    /// Dry run: preview changes without writing files
    #[arg(long, global = true)]
    pub(crate) dry_run: bool,

    #[command(subcommand)]
    pub(crate) command: Commands,
}
