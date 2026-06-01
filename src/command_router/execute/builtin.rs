use super::super::BuiltinOp;
use crate::RenderTarget;
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::write::WriteOp;

pub(super) fn execute_builtin(
    config: &Config,
    builtin: &BuiltinOp,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
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
        } => {
            let mut all_diags = vec![];
            match target {
                RenderTarget::Rfc => all_diags.extend(cmd::render::render(config, None, *dry_run)?),
                RenderTarget::Adr => {
                    all_diags.extend(cmd::render::render_adrs(config, None, *dry_run)?)
                }
                RenderTarget::Work => {
                    all_diags.extend(cmd::render::render_work_items(config, None, *dry_run)?)
                }
                RenderTarget::Changelog => {
                    all_diags.extend(cmd::render::render_changelog(config, *dry_run, *force)?)
                }
                RenderTarget::All => {
                    all_diags.extend(cmd::render::render(config, None, *dry_run)?);
                    all_diags.extend(cmd::render::render_adrs(config, None, *dry_run)?);
                    all_diags.extend(cmd::render::render_work_items(config, None, *dry_run)?);
                }
            }
            Ok(all_diags)
        }
        BuiltinOp::Migrate => cmd::migrate::migrate(config, op),
        BuiltinOp::Verify { guard_ids, work } => {
            Ok(cmd::verify::verify(config, guard_ids, work.as_deref())?)
        }
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
        BuiltinOp::Tui => {
            crate::tui::run(config)?;
            Ok(vec![])
        }
        BuiltinOp::ReleaseCut { version, date } => {
            cmd::lifecycle::cut_release(config, version, date.as_deref(), op)
        }
        BuiltinOp::TagNew { tag } => cmd::tag::tag_new(config, tag, op),
        BuiltinOp::TagDelete { tag } => cmd::tag::tag_delete(config, tag, op),
        BuiltinOp::TagList { output } => cmd::tag::tag_list(config, *output),
        BuiltinOp::LoopStart {
            loop_id,
            work_items,
        } => Ok(cmd::loop_cmd::start(
            config,
            loop_id.as_deref(),
            work_items,
            op,
        )?),
        BuiltinOp::LoopList {
            filter,
            limit,
            output,
        } => Ok(cmd::loop_cmd::list(
            config,
            filter.as_deref(),
            *limit,
            *output,
        )?),
        BuiltinOp::LoopShow { loop_id } => Ok(cmd::loop_cmd::show(config, loop_id)?),
        BuiltinOp::LoopResume {
            loop_id,
            work_items,
        } => Ok(cmd::loop_cmd::resume(
            config,
            loop_id.as_deref(),
            work_items,
        )?),
        BuiltinOp::LoopReplan { loop_id } => Ok(cmd::loop_cmd::replan(config, loop_id, op)?),
        BuiltinOp::LoopAdd {
            loop_id,
            work_items,
        } => Ok(cmd::loop_cmd::add_roots(config, loop_id, work_items, op)?),
        BuiltinOp::LoopRemove {
            loop_id,
            work_items,
        } => Ok(cmd::loop_cmd::remove_roots(
            config, loop_id, work_items, op,
        )?),
        BuiltinOp::LoopRun {
            loop_id,
            work_items,
            max_rounds,
        } => Ok(cmd::loop_cmd::run(
            config,
            loop_id.as_deref(),
            work_items,
            *max_rounds,
            op,
        )?),
    }
}
