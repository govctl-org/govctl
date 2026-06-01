use super::{BuiltinOp, CommandPlan, CreateOp, EditOp, LifecycleOp, Op, Scope};
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::write::WriteOp;
use crate::{ListTarget, NewTarget, OutputFormat, RenderTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShowKind {
    Rfc,
    Clause,
    Adr,
    Work,
    Guard,
}

impl ShowKind {
    fn from_artifact(artifact: cmd::edit::ArtifactType) -> Self {
        match artifact {
            cmd::edit::ArtifactType::Rfc => Self::Rfc,
            cmd::edit::ArtifactType::Clause => Self::Clause,
            cmd::edit::ArtifactType::Adr => Self::Adr,
            cmd::edit::ArtifactType::WorkItem => Self::Work,
            cmd::edit::ArtifactType::Guard => Self::Guard,
        }
    }
}

fn extract_artifact_scope(scope: &Scope) -> anyhow::Result<(cmd::edit::ArtifactType, &str)> {
    match scope {
        Scope::Artifact { artifact, id } => Ok((*artifact, id.as_str())),
        Scope::Target { artifact, id, .. } => Ok((*artifact, id.as_str())),
        Scope::Global | Scope::Collection { .. } => Err(anyhow::anyhow!("expected artifact scope")),
    }
}

fn extract_target_scope(
    scope: &Scope,
) -> anyhow::Result<(
    cmd::edit::ArtifactType,
    &str,
    &cmd::edit::engine::ResolvedTarget,
)> {
    match scope {
        Scope::Target {
            artifact,
            id,
            target,
        } => Ok((*artifact, id.as_str(), target)),
        Scope::Global | Scope::Collection { .. } | Scope::Artifact { .. } => {
            Err(anyhow::anyhow!("expected target scope"))
        }
    }
}

fn extract_collection_scope(scope: &Scope) -> anyhow::Result<ListTarget> {
    match scope {
        Scope::Collection { target } => Ok(*target),
        Scope::Global | Scope::Artifact { .. } | Scope::Target { .. } => {
            Err(anyhow::anyhow!("expected collection scope"))
        }
    }
}

fn execute_builtin(
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
            cmd::verify::verify(config, guard_ids, work.as_deref())
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
        } => cmd::loop_cmd::start(config, loop_id.as_deref(), work_items, op),
        BuiltinOp::LoopList { output } => cmd::loop_cmd::list(config, *output),
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

fn execute_create(
    config: &Config,
    create: &CreateOp,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    match create {
        CreateOp::Rfc { title, id } => cmd::new::create(
            config,
            &NewTarget::Rfc {
                title: title.clone(),
                id: id.clone(),
            },
            op,
        ),
        CreateOp::Clause {
            clause_id,
            title,
            section,
            kind,
        } => cmd::new::create(
            config,
            &NewTarget::Clause {
                clause_id: clause_id.clone(),
                title: title.clone(),
                section: section.clone(),
                kind: *kind,
            },
            op,
        ),
        CreateOp::Adr { title } => cmd::new::create(
            config,
            &NewTarget::Adr {
                title: title.clone(),
            },
            op,
        ),
        CreateOp::Work { title, active } => cmd::new::create(
            config,
            &NewTarget::Work {
                title: title.clone(),
                active: *active,
            },
            op,
        ),
        CreateOp::Guard { title } => cmd::guard::new_guard(config, title, op),
    }
}

fn execute_list(
    plan: &CommandPlan,
    config: &Config,
    filter: Option<&str>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: &[String],
) -> anyhow::Result<Vec<Diagnostic>> {
    cmd::list::list(
        config,
        extract_collection_scope(&plan.scope)?,
        filter,
        limit,
        output,
        tags,
    )
}

fn execute_get(plan: &CommandPlan, config: &Config) -> anyhow::Result<Vec<Diagnostic>> {
    match &plan.scope {
        Scope::Artifact { id, .. } => cmd::edit::get_field(config, id, None),
        Scope::Target { id, target, .. } => {
            let path = target.display_path();
            cmd::edit::get_field(config, id, Some(path.as_str()))
        }
        Scope::Global | Scope::Collection { .. } => {
            Err(anyhow::anyhow!("get requires artifact scope"))
        }
    }
}

fn execute_show(
    plan: &CommandPlan,
    config: &Config,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match ShowKind::from_artifact(artifact) {
        ShowKind::Rfc => cmd::render::show_rfc(config, id, output),
        ShowKind::Clause => cmd::render::show_clause(config, id, output),
        ShowKind::Adr => cmd::render::show_adr(config, id, output),
        ShowKind::Work => cmd::render::show_work(config, id, output),
        ShowKind::Guard => cmd::guard::show_guard(config, id, output),
    }
}

fn execute_edit(
    plan: &CommandPlan,
    config: &Config,
    edit: &EditOp,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    match edit {
        EditOp::Field { action, extras } => {
            let (_, id, target) = extract_target_scope(&plan.scope)?;
            let path = target.display_path();
            let pros = (!extras.pros.is_empty()).then(|| extras.pros.clone());
            let cons = (!extras.cons.is_empty()).then(|| extras.cons.clone());
            cmd::edit::edit_field(cmd::edit::EditFieldRequest {
                config,
                id,
                path: &path,
                action,
                category_override: extras.category,
                pros,
                cons,
                reject_reason: extras.reject_reason.clone(),
                op,
            })
        }
        EditOp::ClauseLegacy {
            text,
            text_file,
            stdin,
        } => {
            let (_, id) = extract_artifact_scope(&plan.scope)?;
            cmd::edit::edit_clause(
                config,
                id,
                text.as_deref(),
                text_file.as_deref(),
                *stdin,
                op,
            )
        }
    }
}

fn execute_lifecycle(
    plan: &CommandPlan,
    config: &Config,
    lifecycle: &LifecycleOp,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match lifecycle {
        LifecycleOp::Bump {
            level,
            summary,
            changes,
        } => cmd::lifecycle::bump(config, id, *level, summary.as_deref(), changes, op),
        LifecycleOp::Finalize { status } => cmd::lifecycle::finalize(config, id, *status, op),
        LifecycleOp::Advance { phase } => cmd::lifecycle::advance(config, id, *phase, op),
        LifecycleOp::Deprecate { force } => cmd::lifecycle::deprecate(config, id, *force, op),
        LifecycleOp::Supersede { by, force } => {
            cmd::lifecycle::supersede(config, id, by, *force, op)
        }
        LifecycleOp::AcceptAdr { force } => {
            debug_assert!(matches!(artifact, cmd::edit::ArtifactType::Adr));
            cmd::lifecycle::accept_adr(config, id, *force, op)
        }
        LifecycleOp::RejectAdr => {
            debug_assert!(matches!(artifact, cmd::edit::ArtifactType::Adr));
            cmd::lifecycle::reject_adr(config, id, op)
        }
        LifecycleOp::MoveWork { file_or_id, status } => {
            cmd::move_::move_item(config, file_or_id, *status, op)
        }
    }
}

fn execute_delete(
    plan: &CommandPlan,
    config: &Config,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match artifact {
        cmd::edit::ArtifactType::Clause => cmd::edit::delete_clause(config, id, force, op),
        cmd::edit::ArtifactType::WorkItem => cmd::edit::delete_work_item(config, id, force, op),
        cmd::edit::ArtifactType::Guard => cmd::guard::delete_guard(config, id, force, op),
        cmd::edit::ArtifactType::Rfc | cmd::edit::ArtifactType::Adr => {
            Err(anyhow::anyhow!("delete is not supported for this artifact"))
        }
    }
}

fn execute_artifact_render(
    plan: &CommandPlan,
    config: &Config,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match artifact {
        cmd::edit::ArtifactType::Rfc => cmd::render::render(config, Some(id), dry_run),
        cmd::edit::ArtifactType::Adr => cmd::render::render_adrs(config, Some(id), dry_run),
        cmd::edit::ArtifactType::WorkItem => {
            cmd::render::render_work_items(config, Some(id), dry_run)
        }
        cmd::edit::ArtifactType::Clause | cmd::edit::ArtifactType::Guard => {
            Err(anyhow::anyhow!("render is not supported for this artifact"))
        }
    }
}

pub(super) fn execute_plan(
    plan: &CommandPlan,
    config: &Config,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    match &plan.op {
        Op::Builtin(builtin) => execute_builtin(config, builtin, op),
        Op::Create(create) => execute_create(config, create, op),
        Op::List {
            filter,
            limit,
            output,
            tags,
        } => execute_list(plan, config, filter.as_deref(), *limit, *output, tags),
        Op::Get => execute_get(plan, config),
        Op::Show { output } => execute_show(plan, config, *output),
        Op::Edit(edit) => execute_edit(plan, config, edit, op),
        Op::Lifecycle(lifecycle) => execute_lifecycle(plan, config, lifecycle, op),
        Op::Delete { force } => execute_delete(plan, config, *force, op),
        Op::RenderArtifact { dry_run } => execute_artifact_render(plan, config, *dry_run),
    }
}
