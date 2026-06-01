use super::{CommandResult, scope::extract_artifact_scope};
use crate::RenderTarget;
use crate::cmd;
use crate::command_router::CommandPlan;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};

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

pub(super) fn execute_global_render(
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

pub(super) fn execute_artifact_render(
    plan: &CommandPlan,
    config: &Config,
    dry_run: bool,
) -> CommandResult {
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
