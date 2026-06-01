use super::super::BuiltinOp;
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::write::WriteOp;

use super::{CommandResult, legacy_command, render_global_target};

pub(super) fn execute_builtin(config: &Config, builtin: &BuiltinOp, op: WriteOp) -> CommandResult {
    match builtin {
        BuiltinOp::Init { force } => {
            legacy_command(cmd::new::init_project(config, *force, op), "init")
        }
        BuiltinOp::InitSkills { force, format, dir } => legacy_command(
            cmd::new::sync_skills(config, *force, format, dir.as_deref(), op),
            "init skills",
        ),
        BuiltinOp::Check { has_active: true } => {
            legacy_command(cmd::check::check_has_active(config), "check active")
        }
        BuiltinOp::Check { has_active: false } => {
            legacy_command(cmd::check::check_all(config), "check")
        }
        BuiltinOp::Status => legacy_command(cmd::status::show_status(config), "status"),
        BuiltinOp::RenderGlobal {
            target,
            dry_run,
            force,
        } => render_global_target(config, *target, *dry_run, *force),
        BuiltinOp::Migrate => legacy_command(cmd::migrate::migrate(config, op), "migrate"),
        BuiltinOp::Verify { guard_ids, work } => {
            cmd::verify::verify(config, guard_ids, work.as_deref())
        }
        BuiltinOp::Describe { context } => {
            legacy_command(cmd::describe::describe(config, *context), "describe")
        }
        BuiltinOp::SelfUpdate { check } => {
            legacy_command(cmd::self_update::self_update(*check), "self-update")
        }
        BuiltinOp::Completions { shell } => {
            use crate::Cli;
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "govctl", &mut std::io::stdout());
            Ok(vec![])
        }
        #[cfg(feature = "tui")]
        BuiltinOp::Tui => {
            crate::tui::run(config).map_err(|err| Diagnostic::from_anyhow(err, "tui"))?;
            Ok(vec![])
        }
        BuiltinOp::ReleaseCut { version, date } => legacy_command(
            cmd::lifecycle::cut_release(config, version, date.as_deref(), op),
            "release cut",
        ),
        BuiltinOp::TagNew { tag } => legacy_command(cmd::tag::tag_new(config, tag, op), "tag new"),
        BuiltinOp::TagDelete { tag } => {
            legacy_command(cmd::tag::tag_delete(config, tag, op), "tag delete")
        }
        BuiltinOp::TagList { output } => {
            legacy_command(cmd::tag::tag_list(config, *output), "tag list")
        }
        BuiltinOp::LoopStart {
            loop_id,
            work_items,
        } => cmd::loop_cmd::start(config, loop_id.as_deref(), work_items, op),
        BuiltinOp::LoopList {
            filter,
            limit,
            output,
        } => cmd::loop_cmd::list(config, filter.as_deref(), *limit, *output),
        BuiltinOp::LoopShow { loop_id } => cmd::loop_cmd::show(config, loop_id),
        BuiltinOp::LoopResume {
            loop_id,
            work_items,
        } => cmd::loop_cmd::resume(config, loop_id.as_deref(), work_items),
        BuiltinOp::LoopReplan { loop_id } => cmd::loop_cmd::replan(config, loop_id, op),
        BuiltinOp::LoopAdd {
            loop_id,
            work_items,
        } => cmd::loop_cmd::add_roots(config, loop_id, work_items, op),
        BuiltinOp::LoopRemove {
            loop_id,
            work_items,
        } => cmd::loop_cmd::remove_roots(config, loop_id, work_items, op),
        BuiltinOp::LoopRun {
            loop_id,
            work_items,
            max_rounds,
        } => cmd::loop_cmd::run(config, loop_id.as_deref(), work_items, *max_rounds, op),
    }
}
