use super::super::BuiltinOp;
use crate::cmd;
use crate::config::Config;
use crate::write::WriteOp;

use super::{CommandResult, render::execute_global_render};

pub(super) fn execute_builtin(config: &Config, builtin: &BuiltinOp, op: WriteOp) -> CommandResult {
    match builtin {
        BuiltinOp::Init { force } => cmd::new::init_project(config, *force, op),
        BuiltinOp::InitSkills { force, format, dir } => {
            cmd::new::sync_skills(config, *force, format, dir.as_deref(), op)
        }
        BuiltinOp::Check { has_active: true } => cmd::check::check_has_active(config),
        BuiltinOp::Check { has_active: false } => cmd::check::check_all(config),
        BuiltinOp::Status => cmd::status::show_status(config),
        BuiltinOp::RenderGlobal {
            target,
            dry_run,
            force,
        } => execute_global_render(config, *target, *dry_run, *force),
        BuiltinOp::Migrate => cmd::migrate::migrate(config, op),
        BuiltinOp::Verify { guard_ids, work } => {
            cmd::verify::verify(config, guard_ids, work.as_deref())
        }
        BuiltinOp::Search {
            query,
            types,
            tags,
            limit,
            output,
            reindex,
        } => cmd::search::search(config, query, types, tags, *limit, *output, *reindex),
        BuiltinOp::Describe { context } => cmd::describe::describe(config, *context),
        BuiltinOp::SelfUpdate { check } => cmd::self_update::self_update(*check),
        BuiltinOp::Completions { shell } => {
            use crate::Cli;
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "govctl", &mut std::io::stdout());
            Ok(vec![])
        }
        #[cfg(feature = "tui")]
        BuiltinOp::Tui => crate::tui::run(config).map(|()| vec![]),
        BuiltinOp::ReleaseCut { version, date } => {
            cmd::lifecycle::cut_release(config, version, date.as_deref(), op)
        }
        BuiltinOp::ReleaseUndo { expected_version } => {
            cmd::lifecycle::undo_release(config, expected_version, op)
        }
        BuiltinOp::TagNew { tag } => cmd::tag::tag_new(config, tag, op),
        BuiltinOp::TagDelete { tag } => cmd::tag::tag_delete(config, tag, op),
        BuiltinOp::TagList { output } => cmd::tag::tag_list(config, *output),
        BuiltinOp::LoopStart { loop_id, work_ids } => {
            cmd::loop_cmd::start(config, loop_id.as_deref(), work_ids, op)
        }
        BuiltinOp::LoopList {
            filter,
            limit,
            output,
        } => cmd::loop_cmd::list(config, filter.as_deref(), *limit, *output),
        BuiltinOp::LoopShow { loop_id } => cmd::loop_cmd::show(config, loop_id),
        BuiltinOp::LoopResume { loop_id } => cmd::loop_cmd::resume(config, loop_id),
        BuiltinOp::LoopReplan { loop_id } => cmd::loop_cmd::replan(config, loop_id, op),
        BuiltinOp::LoopAdd {
            loop_id,
            field,
            value,
        } => cmd::loop_cmd::add_work_item(config, loop_id, field, value, op),
        BuiltinOp::LoopRemove {
            loop_id,
            field,
            value,
        } => cmd::loop_cmd::remove_work_item(config, loop_id, field, value, op),
        BuiltinOp::LoopRun {
            loop_id,
            target_work_ids,
        } => cmd::loop_cmd::run(config, loop_id, target_work_ids, op),
    }
}
