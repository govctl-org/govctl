mod builtin;
mod render;
mod scope;

use super::{CommandPlan, CreateOp, EditOp, LifecycleOp, Op, Scope};
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::write::WriteOp;
use crate::{GetOutputFormat, ListOutputFormat, NewTarget, ShowOutputFormat};
use builtin::execute_builtin;
use render::execute_artifact_render;
use scope::{ShowKind, extract_artifact_scope, extract_collection_scope, extract_target_scope};

type CommandResult = DiagnosticResult<Diagnostics>;

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
        } => {
            // [[RFC-0010:C-ARTIFACT-CLAIM]]: clause authoring is content work;
            // a foreign live claim on the parent RFC warns without blocking.
            let mut diags = claim_content_warnings(config, rfc_id_of_clause(clause_id), op);
            diags.extend(cmd::new::create(
                config,
                &NewTarget::Clause {
                    clause_id: clause_id.clone(),
                    title: title.clone(),
                    section: section.clone(),
                    kind: *kind,
                },
                op,
            )?);
            Ok(diags)
        }
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
    output: Option<ListOutputFormat>,
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

fn execute_get(
    plan: &CommandPlan,
    config: &Config,
    output: Option<GetOutputFormat>,
) -> CommandResult {
    match &plan.scope {
        Scope::Artifact { id, .. } => cmd::edit::get_field(config, id, None, output),
        Scope::Target { id, target, .. } => {
            let path = target.display_path();
            cmd::edit::get_field(config, id, Some(path.as_str()), output)
        }
        Scope::Global | Scope::Collection { .. } => Err(Diagnostic::new(
            DiagnosticCode::E0821InvalidCommandScope,
            "get requires artifact scope",
            "command router",
        )),
    }
}

fn execute_show(
    plan: &CommandPlan,
    config: &Config,
    output: ShowOutputFormat,
    history: bool,
) -> CommandResult {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match ShowKind::from_artifact(artifact) {
        ShowKind::Rfc => cmd::render::show_rfc(config, id, output, history),
        ShowKind::Clause => cmd::render::show_clause(config, id, output, history),
        ShowKind::Adr => cmd::render::show_adr(config, id, output, history),
        ShowKind::Work => cmd::render::show_work(config, id, output, history),
        ShowKind::Guard => cmd::guard::show_guard(config, id, output, history),
        ShowKind::Conformance => cmd::conformance::show(config, id, output, history),
    }
}

fn execute_edit(plan: &CommandPlan, config: &Config, edit: &EditOp, op: WriteOp) -> CommandResult {
    match edit {
        EditOp::Field { action } => {
            let (artifact, id, target) = extract_target_scope(&plan.scope)?;
            let path = target.display_path();
            // [[RFC-0010:C-ARTIFACT-CLAIM]]: content edits on an RFC claimed
            // by another workspace warn without blocking.
            let mut diags = match artifact {
                cmd::edit::ArtifactType::Rfc => claim_content_warnings(config, id, op),
                cmd::edit::ArtifactType::Clause => {
                    claim_content_warnings(config, rfc_id_of_clause(id), op)
                }
                _ => vec![],
            };
            diags.extend(cmd::edit::edit_field(cmd::edit::EditFieldRequest {
                config,
                id,
                path: &path,
                action,
                op,
            })?);
            Ok(diags)
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
    // [[RFC-0010:C-ARTIFACT-CLAIM]]: RFC version-semantics operations acquire
    // exclusive claims before mutating and fail without mutation when another
    // workspace holds a live claim; clause lifecycle operations are content
    // work and warn without blocking.
    let mut diags = coordinate_claims(config, artifact, id, lifecycle, op)?;
    diags.extend(match lifecycle {
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
    }?);
    Ok(diags)
}

/// Claim coordination for lifecycle operations — [[RFC-0010:C-ARTIFACT-CLAIM]].
///
/// RFC-scoped bump/finalize/advance/deprecate acquire a claim on the RFC;
/// RFC supersede acquires claims on both the superseded and the superseding
/// RFC. Clause-scoped deprecate/supersede are content operations on the
/// parent RFC and receive soft warnings only.
fn coordinate_claims(
    config: &Config,
    artifact: cmd::edit::ArtifactType,
    id: &str,
    lifecycle: &LifecycleOp,
    op: WriteOp,
) -> CommandResult {
    let is_rfc = artifact == cmd::edit::ArtifactType::Rfc;
    let hard_ids: Vec<&str> = match lifecycle {
        LifecycleOp::Bump { .. } | LifecycleOp::Finalize { .. } | LifecycleOp::Advance { .. }
            if is_rfc =>
        {
            vec![id]
        }
        LifecycleOp::Deprecate { .. } if is_rfc => vec![id],
        LifecycleOp::Supersede { by, .. } if is_rfc => vec![id, by],
        _ => vec![],
    };
    let soft_ids: Vec<&str> = match lifecycle {
        LifecycleOp::Deprecate { .. } | LifecycleOp::Supersede { .. }
            if artifact == cmd::edit::ArtifactType::Clause =>
        {
            vec![rfc_id_of_clause(id)]
        }
        _ => vec![],
    };
    if hard_ids.is_empty() && soft_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut session = crate::registry::ClaimSession::begin(config, op.is_preview());
    session.acquire(&hard_ids)?;
    let mut diags = session.foreign_claim_warnings(&soft_ids);
    diags.extend(session.into_warnings());
    Ok(diags)
}

/// Parent RFC of a clause ID (`RFC-0001:C-FOO` → `RFC-0001`).
fn rfc_id_of_clause(id: &str) -> &str {
    id.split(':').next().unwrap_or(id)
}

/// Soft claim warnings for content operations on an RFC —
/// [[RFC-0010:C-ARTIFACT-CLAIM]].
fn claim_content_warnings(config: &Config, rfc_id: &str, op: WriteOp) -> Diagnostics {
    let mut session = crate::registry::ClaimSession::begin(config, op.is_preview());
    let mut diags = session.foreign_claim_warnings(&[rfc_id]);
    diags.extend(session.into_warnings());
    diags
}

fn execute_delete(plan: &CommandPlan, config: &Config, force: bool, op: WriteOp) -> CommandResult {
    let (artifact, id) = extract_artifact_scope(&plan.scope)?;
    match artifact {
        cmd::edit::ArtifactType::Clause => {
            // [[RFC-0010:C-ARTIFACT-CLAIM]]: clause deletion is content work;
            // a foreign live claim on the parent RFC warns without blocking.
            let mut diags = claim_content_warnings(config, rfc_id_of_clause(id), op);
            diags.extend(cmd::edit::delete_clause(config, id, force, op)?);
            Ok(diags)
        }
        cmd::edit::ArtifactType::WorkItem => cmd::edit::delete_work_item(config, id, force, op),
        cmd::edit::ArtifactType::Guard => cmd::guard::delete_guard(config, id, force, op),
        cmd::edit::ArtifactType::Conformance => cmd::conformance::delete(config, id, force, op),
        cmd::edit::ArtifactType::Rfc | cmd::edit::ArtifactType::Adr => Err(Diagnostic::new(
            DiagnosticCode::E0822UnsupportedOperation,
            "delete is not supported for this artifact",
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
        Op::Get { output } => execute_get(plan, config, *output),
        Op::Show { output, history } => execute_show(plan, config, *output, *history),
        Op::Edit(edit) => execute_edit(plan, config, edit, op),
        Op::Lifecycle(lifecycle) => execute_lifecycle(plan, config, lifecycle, op),
        Op::Delete { force } => execute_delete(plan, config, *force, op),
        Op::RenderArtifact { dry_run } => execute_artifact_render(plan, config, *dry_run),
    }
}
