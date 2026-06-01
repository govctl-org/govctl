mod builtin;
mod scope;

use super::{CommandPlan, CreateOp, EditOp, LifecycleOp, Op, Scope};
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::write::WriteOp;
use crate::{NewTarget, OutputFormat, RenderTarget};
use builtin::execute_builtin;
use scope::{ShowKind, extract_artifact_scope, extract_collection_scope, extract_target_scope};

type CommandResult = DiagnosticResult<Diagnostics>;

fn legacy_command(result: anyhow::Result<Diagnostics>, context: &str) -> CommandResult {
    result.map_err(|err| Diagnostic::from_anyhow(err, context))
}

fn render_rfc(config: &Config, id: Option<&str>, dry_run: bool) -> CommandResult {
    cmd::render::render(config, id, dry_run)
}

fn render_adr(config: &Config, id: Option<&str>, dry_run: bool) -> CommandResult {
    cmd::render::render_adrs(config, id, dry_run)
}

fn render_work(config: &Config, id: Option<&str>, dry_run: bool) -> CommandResult {
    cmd::render::render_work_items(config, id, dry_run)
}

fn render_changelog(config: &Config, dry_run: bool, force: bool) -> CommandResult {
    cmd::render::render_changelog(config, dry_run, force)
}

fn render_global_target(
    config: &Config,
    target: RenderTarget,
    dry_run: bool,
    force: bool,
) -> CommandResult {
    let mut all_diags = vec![];
    match target {
        RenderTarget::Rfc => all_diags.extend(render_rfc(config, None, dry_run)?),
        RenderTarget::Adr => all_diags.extend(render_adr(config, None, dry_run)?),
        RenderTarget::Work => all_diags.extend(render_work(config, None, dry_run)?),
        RenderTarget::Changelog => all_diags.extend(render_changelog(config, dry_run, force)?),
        RenderTarget::All => {
            all_diags.extend(render_rfc(config, None, dry_run)?);
            all_diags.extend(render_adr(config, None, dry_run)?);
            all_diags.extend(render_work(config, None, dry_run)?);
        }
    }
    Ok(all_diags)
}

fn execute_create(config: &Config, create: &CreateOp, op: WriteOp) -> CommandResult {
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
) -> CommandResult {
    cmd::list::list(
        config,
        extract_collection_scope(&plan.scope)?,
        filter,
        limit,
        output,
        tags,
    )
}

fn execute_get(plan: &CommandPlan, config: &Config) -> CommandResult {
    match &plan.scope {
        Scope::Artifact { id, .. } => cmd::edit::get_field(config, id, None),
        Scope::Target { id, target, .. } => {
            let path = target.display_path();
            cmd::edit::get_field(config, id, Some(path.as_str()))
        }
        Scope::Global | Scope::Collection { .. } => Err(Diagnostic::new(
            DiagnosticCode::E0821InvalidCommandScope,
            "get requires artifact scope",
            "command router",
        )),
    }
}

fn execute_show(plan: &CommandPlan, config: &Config, output: OutputFormat) -> CommandResult {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match ShowKind::from_artifact(artifact) {
        ShowKind::Rfc => cmd::render::show_rfc(config, id, output),
        ShowKind::Clause => cmd::render::show_clause(config, id, output),
        ShowKind::Adr => cmd::render::show_adr(config, id, output),
        ShowKind::Work => cmd::render::show_work(config, id, output),
        ShowKind::Guard => cmd::guard::show_guard(config, id, output),
    }
}

fn execute_edit(plan: &CommandPlan, config: &Config, edit: &EditOp, op: WriteOp) -> CommandResult {
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
) -> CommandResult {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match lifecycle {
        LifecycleOp::Bump {
            level,
            summary,
            changes,
        } => cmd::lifecycle::bump(config, id, *level, summary.as_deref(), changes, op),
        LifecycleOp::Finalize { status } => cmd::lifecycle::finalize(config, id, *status, op),
        LifecycleOp::Advance { phase } => cmd::lifecycle::advance(config, id, *phase, op),
        LifecycleOp::Deprecate { force } => {
            cmd::lifecycle::deprecate(config, id, *force, op)
        }
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

fn execute_delete(plan: &CommandPlan, config: &Config, force: bool, op: WriteOp) -> CommandResult {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match artifact {
        cmd::edit::ArtifactType::Clause => cmd::edit::delete_clause(config, id, force, op),
        cmd::edit::ArtifactType::WorkItem => cmd::edit::delete_work_item(config, id, force, op),
        cmd::edit::ArtifactType::Guard => cmd::guard::delete_guard(config, id, force, op),
        cmd::edit::ArtifactType::Rfc | cmd::edit::ArtifactType::Adr => Err(Diagnostic::new(
            DiagnosticCode::E0822UnsupportedOperation,
            "delete is not supported for this artifact",
            id,
        )),
    }
}

fn execute_artifact_render(plan: &CommandPlan, config: &Config, dry_run: bool) -> CommandResult {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match artifact {
        cmd::edit::ArtifactType::Rfc => render_rfc(config, Some(id), dry_run),
        cmd::edit::ArtifactType::Adr => render_adr(config, Some(id), dry_run),
        cmd::edit::ArtifactType::WorkItem => render_work(config, Some(id), dry_run),
        cmd::edit::ArtifactType::Clause | cmd::edit::ArtifactType::Guard => Err(Diagnostic::new(
            DiagnosticCode::E0822UnsupportedOperation,
            "render is not supported for this artifact",
            id,
        )),
    }
}

pub(super) fn execute_plan(plan: &CommandPlan, config: &Config, op: WriteOp) -> CommandResult {
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
