//! Command planning for unified routing semantics.
//!
//! This module compiles parsed CLI syntax into semantic execution plans built
//! from `Scope + Op`. The planner is the single normalization point for both
//! canonical and compatibility command forms.

use crate::cmd;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChangelogCategory, ClauseKind, RfcPhase, WorkItemStatus};
use crate::write::{BumpLevel, WriteOp};
use crate::{
    Commands, EditActionArgs, FinalizeStatus, ListTarget, NewTarget, OutputFormat, RenderTarget,
    TickStatus,
};
use std::path::PathBuf;

pub(crate) type OwnedMatchOptions = cmd::edit::MatchOptionsOwned;
pub(crate) type OwnedEditAction = cmd::edit::OwnedEditAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Global,
    Collection {
        target: ListTarget,
    },
    Artifact {
        artifact: cmd::edit::ArtifactType,
        id: String,
    },
    Target {
        artifact: cmd::edit::ArtifactType,
        id: String,
        target: cmd::edit::engine::ResolvedTarget,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditExtras {
    pub category: Option<ChangelogCategory>,
    pub scope: Option<String>,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub reject_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BuiltinOp {
    Init {
        force: bool,
    },
    InitSkills {
        force: bool,
        format: crate::SkillFormat,
        dir: Option<std::path::PathBuf>,
    },
    Check {
        #[allow(dead_code)]
        deny_warnings: bool,
        has_active: bool,
    },
    Status,
    RenderGlobal {
        target: RenderTarget,
        dry_run: bool,
        force: bool,
    },
    Migrate,
    Verify {
        guard_ids: Vec<String>,
        work: Option<String>,
    },
    Describe {
        context: bool,
        #[allow(dead_code)]
        output: String,
    },
    Completions {
        shell: clap_complete::Shell,
    },
    #[cfg(feature = "tui")]
    Tui,
    ReleaseCut {
        version: String,
        date: Option<String>,
    },
    TagNew {
        tag: String,
    },
    TagDelete {
        tag: String,
    },
    TagList {
        output: crate::OutputFormat,
    },
}

#[derive(Debug, Clone)]
pub enum CreateOp {
    Rfc {
        title: String,
        id: Option<String>,
    },
    Clause {
        clause_id: String,
        title: String,
        section: String,
        kind: ClauseKind,
    },
    Adr {
        title: String,
    },
    Work {
        title: String,
        active: bool,
    },
    Guard {
        title: String,
    },
}

#[derive(Debug, Clone)]
pub enum EditOp {
    Field {
        action: OwnedEditAction,
        extras: EditExtras,
    },
    ClauseLegacy {
        text: Option<String>,
        text_file: Option<PathBuf>,
        stdin: bool,
    },
}

#[derive(Debug, Clone)]
pub enum LifecycleOp {
    Bump {
        level: Option<BumpLevel>,
        summary: Option<String>,
        changes: Vec<String>,
    },
    Finalize {
        status: FinalizeStatus,
    },
    Advance {
        phase: RfcPhase,
    },
    Deprecate {
        force: bool,
    },
    Supersede {
        by: String,
        force: bool,
    },
    AcceptAdr,
    RejectAdr,
    MoveWork {
        file_or_id: PathBuf,
        status: WorkItemStatus,
    },
}

#[derive(Debug, Clone)]
pub enum Op {
    Builtin(BuiltinOp),
    Create(CreateOp),
    List {
        filter: Option<String>,
        limit: Option<usize>,
        output: OutputFormat,
        /// Tags to filter by (artifact must have ALL specified tags) — [[RFC-0002:C-CRUD-VERBS]]
        tags: Vec<String>,
    },
    Get,
    Show {
        output: OutputFormat,
    },
    Edit(EditOp),
    Lifecycle(LifecycleOp),
    Delete {
        force: bool,
    },
    RenderArtifact {
        dry_run: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockDisposition {
    None,
    GovRootExclusive,
}

#[derive(Debug, Clone)]
pub struct CommandPlan {
    pub scope: Scope,
    pub op: Op,
}

impl CommandPlan {
    fn new(scope: Scope, op: Op) -> Self {
        Self { scope, op }
    }

    pub fn lock_disposition(&self) -> LockDisposition {
        match &self.op {
            Op::Builtin(BuiltinOp::Check { .. })
            | Op::Builtin(BuiltinOp::Status)
            | Op::Builtin(BuiltinOp::Verify { .. })
            | Op::Builtin(BuiltinOp::Describe { .. })
            | Op::Builtin(BuiltinOp::Completions { .. })
            | Op::Builtin(BuiltinOp::TagList { .. })
            | Op::Get
            | Op::List { .. }
            | Op::Show { .. } => LockDisposition::None,
            #[cfg(feature = "tui")]
            Op::Builtin(BuiltinOp::Tui) => LockDisposition::None,
            _ => LockDisposition::GovRootExclusive,
        }
    }

    pub fn execute(&self, config: &Config, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
        execute_plan(self, config, op)
    }
}

fn conflicting_edit_flag_error(action: &str, flag: &str) -> anyhow::Error {
    Diagnostic::new(
        DiagnosticCode::E0802ConflictingArgs,
        format!("Cannot use {flag} with --{action}"),
        "edit action",
    )
    .into()
}

fn reject_selector_flags_for_value_action(
    action: &str,
    args: &EditActionArgs,
) -> anyhow::Result<()> {
    if args.at.is_some() {
        return Err(conflicting_edit_flag_error(action, "--at"));
    }
    if args.exact {
        return Err(conflicting_edit_flag_error(action, "--exact"));
    }
    if args.regex {
        return Err(conflicting_edit_flag_error(action, "--regex"));
    }
    if args.all {
        return Err(conflicting_edit_flag_error(action, "--all"));
    }
    Ok(())
}

pub(crate) fn owned_edit_action(args: &EditActionArgs) -> anyhow::Result<OwnedEditAction> {
    let action_count = usize::from(args.set.is_some())
        + usize::from(args.add.is_some())
        + usize::from(args.tick.is_some())
        + usize::from(args.remove.is_some());

    if action_count == 0 {
        // When --stdin is present with no explicit action, infer --set
        if args.stdin {
            reject_selector_flags_for_value_action("set (inferred from --stdin)", args)?;
            return Ok(OwnedEditAction::Set {
                value: Some(None),
                stdin: true,
            });
        }
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "exactly one edit action is required",
            "edit action",
        )
        .into());
    }

    if action_count > 1 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot use multiple edit actions at once",
            "edit action",
        )
        .into());
    }

    if let Some(value) = &args.set {
        reject_selector_flags_for_value_action("set", args)?;
        return Ok(OwnedEditAction::Set {
            value: Some(value.clone()),
            stdin: args.stdin,
        });
    }
    if let Some(value) = &args.add {
        reject_selector_flags_for_value_action("add", args)?;
        return Ok(OwnedEditAction::Add {
            value: Some(value.clone()),
            stdin: args.stdin,
        });
    }
    if let Some(status) = args.tick {
        if args.stdin {
            return Err(conflicting_edit_flag_error("tick", "--stdin"));
        }
        if args.all {
            return Err(Diagnostic::new(
                DiagnosticCode::E0802ConflictingArgs,
                "Cannot use --all with --tick; tick requires a single target",
                "edit action",
            )
            .into());
        }
        return Ok(OwnedEditAction::Tick {
            match_opts: OwnedMatchOptions {
                pattern: None,
                at: args.at,
                exact: args.exact,
                regex: args.regex,
                all: args.all,
            },
            status,
        });
    }
    if args.remove.is_some() {
        if args.stdin {
            return Err(conflicting_edit_flag_error("remove", "--stdin"));
        }
        return Ok(OwnedEditAction::Remove {
            match_opts: OwnedMatchOptions {
                pattern: args.remove.clone().flatten(),
                at: args.at,
                exact: args.exact,
                regex: args.regex,
                all: args.all,
            },
        });
    }
    unreachable!("action_count guarantees exactly one action branch")
}

fn artifact_scope(artifact: cmd::edit::ArtifactType, id: &str) -> Scope {
    Scope::Artifact {
        artifact,
        id: id.to_string(),
    }
}

fn resolve_scope(id: &str, field: Option<&str>) -> anyhow::Result<Scope> {
    let plan = cmd::edit::engine::plan_request(id, field)?;
    Ok(match plan.target {
        Some(target) => Scope::Target {
            artifact: plan.artifact,
            id: id.to_string(),
            target,
        },
        None => artifact_scope(plan.artifact, id),
    })
}

fn global(op: Op) -> CommandPlan {
    CommandPlan::new(Scope::Global, op)
}

fn collection(target: ListTarget, op: Op) -> CommandPlan {
    CommandPlan::new(Scope::Collection { target }, op)
}

pub(crate) fn artifact(artifact: cmd::edit::ArtifactType, id: &str, op: Op) -> CommandPlan {
    CommandPlan::new(artifact_scope(artifact, id), op)
}

fn target(id: &str, field: Option<&str>, op: Op) -> anyhow::Result<CommandPlan> {
    Ok(CommandPlan::new(resolve_scope(id, field)?, op))
}

fn edit_op_with_extras(action: OwnedEditAction, extras: EditExtras) -> Op {
    Op::Edit(EditOp::Field { action, extras })
}

pub(crate) fn set_action(value: Option<String>, stdin: bool) -> OwnedEditAction {
    OwnedEditAction::Set {
        value: Some(value),
        stdin,
    }
}

pub(crate) fn add_action(value: Option<String>, stdin: bool) -> OwnedEditAction {
    OwnedEditAction::Add {
        value: Some(value),
        stdin,
    }
}

pub(crate) fn remove_action(match_opts: OwnedMatchOptions) -> OwnedEditAction {
    OwnedEditAction::Remove { match_opts }
}

pub(crate) fn tick_action(match_opts: OwnedMatchOptions, status: TickStatus) -> OwnedEditAction {
    OwnedEditAction::Tick { match_opts, status }
}

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
        BuiltinOp::Check {
            deny_warnings: _,
            has_active: true,
        } => cmd::check::check_has_active(config),
        BuiltinOp::Check {
            deny_warnings: _,
            has_active: false,
        } => cmd::check::check_all(config),
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
        BuiltinOp::Describe { context, output: _ } => cmd::describe::describe(config, *context),
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
            cmd::edit::edit_field(
                config,
                id,
                &path,
                action,
                extras.category,
                extras.scope.as_deref(),
                pros,
                cons,
                extras.reject_reason.clone(),
                op,
            )
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
        LifecycleOp::AcceptAdr => {
            debug_assert!(matches!(artifact, cmd::edit::ArtifactType::Adr));
            cmd::lifecycle::accept_adr(config, id, op)
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

fn execute_plan(
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

pub(crate) fn plan_create(collection_target: ListTarget, create: CreateOp) -> CommandPlan {
    collection(collection_target, Op::Create(create))
}

pub(crate) fn plan_list(
    target_kind: ListTarget,
    filter: Option<String>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: Vec<String>,
) -> CommandPlan {
    collection(
        target_kind,
        Op::List {
            filter,
            limit,
            output,
            tags,
        },
    )
}

pub(crate) fn plan_get(id: &str, field: Option<&str>) -> anyhow::Result<CommandPlan> {
    target(id, field, Op::Get)
}

pub(crate) fn plan_show(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    output: OutputFormat,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Show { output })
}

pub(crate) fn plan_edit(
    id: &str,
    field: &str,
    action: OwnedEditAction,
    extras: EditExtras,
) -> anyhow::Result<CommandPlan> {
    target(id, Some(field), edit_op_with_extras(action, extras))
}

pub(crate) fn plan_lifecycle(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    lifecycle: LifecycleOp,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Lifecycle(lifecycle))
}

pub(crate) fn plan_artifact_render(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    dry_run: bool,
) -> CommandPlan {
    artifact(artifact_type, id, Op::RenderArtifact { dry_run })
}

pub(crate) fn plan_delete(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    force: bool,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Delete { force })
}

impl CommandPlan {
    pub fn from_parsed(cmd: &Commands, global_dry_run: bool) -> anyhow::Result<Self> {
        use crate::resource_plan::ToPlan;

        match cmd {
            Commands::Init { force } => Ok(global(Op::Builtin(BuiltinOp::Init { force: *force }))),
            Commands::InitSkills { force, format, dir } => {
                Ok(global(Op::Builtin(BuiltinOp::InitSkills {
                    force: *force,
                    format: format.clone(),
                    dir: dir.clone(),
                })))
            }
            Commands::Check {
                deny_warnings,
                has_active,
            } => Ok(global(Op::Builtin(BuiltinOp::Check {
                deny_warnings: *deny_warnings,
                has_active: *has_active,
            }))),
            Commands::Status => Ok(global(Op::Builtin(BuiltinOp::Status))),
            Commands::Render {
                target,
                dry_run,
                force,
            } => Ok(global(Op::Builtin(BuiltinOp::RenderGlobal {
                target: *target,
                dry_run: global_dry_run || *dry_run,
                force: *force,
            }))),
            Commands::Migrate => Ok(global(Op::Builtin(BuiltinOp::Migrate))),
            Commands::Verify { guard_ids, work } => Ok(global(Op::Builtin(BuiltinOp::Verify {
                guard_ids: guard_ids.clone(),
                work: work.clone(),
            }))),
            Commands::Describe { context, output } => {
                Ok(global(Op::Builtin(BuiltinOp::Describe {
                    context: *context,
                    output: output.clone(),
                })))
            }
            Commands::Completions { shell } => Ok(global(Op::Builtin(BuiltinOp::Completions {
                shell: *shell,
            }))),
            #[cfg(feature = "tui")]
            Commands::Tui => Ok(global(Op::Builtin(BuiltinOp::Tui))),
            Commands::Rfc { command } => command.to_plan(),
            Commands::Clause { command } => command.to_plan(),
            Commands::Adr { command } => command.to_plan(),
            Commands::Work { command } => command.to_plan(),
            Commands::Guard { command } => command.to_plan(),
            Commands::Release { version, date } => Ok(global(Op::Builtin(BuiltinOp::ReleaseCut {
                version: version.clone(),
                date: date.clone(),
            }))),
            Commands::Tag { command } => match command {
                crate::TagCommand::New { tag } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagNew { tag: tag.clone() })))
                }
                crate::TagCommand::Delete { tag } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagDelete {
                        tag: tag.clone(),
                    })))
                }
                crate::TagCommand::List { output } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagList { output: *output })))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_plan::ToPlan;
    use crate::{ClauseCommand, TickStatus, WorkTickStatus};
    use clap::{Parser, error::ErrorKind};

    #[test]
    fn test_owned_edit_action_requires_exactly_one_action() {
        let err = owned_edit_action(&EditActionArgs {
            set: None,
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
        })
        .expect_err("missing action should fail");

        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0801MissingRequiredArg);
    }

    #[test]
    fn test_from_clause_command_uses_canonical_edit_when_path_is_present() {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: Some("text".to_string()),
            set: Some(Some("Updated".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        };

        let plan = cmd.to_plan().expect("canonical edit");
        assert!(matches!(
            plan.scope,
            Scope::Target {
                artifact: cmd::edit::ArtifactType::Clause,
                ..
            }
        ));
        match plan.op {
            Op::Edit(EditOp::Field { action, .. }) => match action {
                OwnedEditAction::Set { value, stdin } => {
                    assert_eq!(value.as_ref(), Some(&Some("Updated".to_string())));
                    assert!(!stdin);
                }
                other => panic!("expected set action, got {other:?}"),
            },
            other => panic!("expected field edit, got {other:?}"),
        }
    }

    #[test]
    fn test_from_clause_command_requires_path_for_canonical_flags() {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: None,
            set: Some(Some("Updated".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        };

        let err = cmd.to_plan().expect_err("missing path");
        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0801MissingRequiredArg);
    }

    #[test]
    fn test_from_clause_command_uses_legacy_edit_without_canonical_flags() {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: None,
            set: None,
            add: None,
            remove: None,
            tick: None,
            stdin: true,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        };

        let plan = cmd.to_plan().expect("legacy edit");
        assert!(matches!(
            plan.scope,
            Scope::Artifact {
                artifact: cmd::edit::ArtifactType::Clause,
                ..
            }
        ));
        match plan.op {
            Op::Edit(EditOp::ClauseLegacy {
                text,
                text_file,
                stdin,
            }) => {
                assert!(text.is_none());
                assert!(text_file.is_none());
                assert!(stdin);
            }
            other => panic!("expected legacy clause edit, got {other:?}"),
        }
    }

    #[test]
    fn test_owned_edit_action_builds_tick_match_options() {
        let action = owned_edit_action(&EditActionArgs {
            set: None,
            add: None,
            remove: None,
            tick: Some(TickStatus::Done),
            stdin: false,
            at: Some(2),
            exact: true,
            regex: false,
            all: false,
        })
        .expect("tick action");

        match action {
            OwnedEditAction::Tick { match_opts, status } => {
                assert!(matches!(status, TickStatus::Done));
                assert_eq!(match_opts.at, Some(2));
                assert!(match_opts.exact);
            }
            other => panic!("expected tick action, got {other:?}"),
        }
    }

    #[test]
    fn test_owned_edit_action_rejects_tick_all_combination() {
        let err = owned_edit_action(&EditActionArgs {
            set: None,
            add: None,
            remove: None,
            tick: Some(TickStatus::Done),
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: true,
        })
        .expect_err("tick with --all should fail");

        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    }

    #[test]
    fn test_owned_edit_action_rejects_multiple_actions() {
        let err = owned_edit_action(&EditActionArgs {
            set: Some(Some("x".to_string())),
            add: Some(Some("y".to_string())),
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
        })
        .expect_err("multiple actions should fail");

        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    }

    #[test]
    fn test_owned_edit_action_preserves_explicit_empty_strings() {
        let set = owned_edit_action(&EditActionArgs {
            set: Some(Some(String::new())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
        })
        .expect("set action");
        match set {
            OwnedEditAction::Set { value, stdin } => {
                assert_eq!(value.as_ref(), Some(&Some(String::new())));
                assert!(!stdin);
            }
            other => panic!("expected set action, got {other:?}"),
        }

        let add = owned_edit_action(&EditActionArgs {
            set: None,
            add: Some(Some(String::new())),
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
        })
        .expect("add action");
        match add {
            OwnedEditAction::Add { value, stdin } => {
                assert_eq!(value.as_ref(), Some(&Some(String::new())));
                assert!(!stdin);
            }
            other => panic!("expected add action, got {other:?}"),
        }

        let remove = owned_edit_action(&EditActionArgs {
            set: None,
            add: None,
            remove: Some(Some(String::new())),
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
        })
        .expect("remove action");
        match remove {
            OwnedEditAction::Remove { match_opts } => {
                assert_eq!(match_opts.pattern.as_deref(), Some(""));
            }
            other => panic!("expected remove action, got {other:?}"),
        }
    }

    #[test]
    fn test_edit_plans_are_mutating() {
        let plan = crate::RfcCommand::Edit(crate::CommonEditArgs {
            id: "RFC-0001".to_string(),
            path: "title".to_string(),
            action: EditActionArgs {
                set: Some(Some("X".to_string())),
                add: None,
                remove: None,
                tick: None,
                stdin: false,
                at: None,
                exact: false,
                regex: false,
                all: false,
            },
        })
        .to_plan()
        .expect("rfc edit");
        assert!(matches!(plan.scope, Scope::Target { .. }));
        assert!(matches!(plan.op, Op::Edit(EditOp::Field { .. })));
        assert_eq!(plan.lock_disposition(), LockDisposition::GovRootExclusive);

        let plan = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: None,
            set: None,
            add: None,
            remove: None,
            tick: None,
            stdin: true,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        }
        .to_plan()
        .expect("legacy edit");
        assert!(matches!(plan.op, Op::Edit(EditOp::ClauseLegacy { .. })));
        assert_eq!(plan.lock_disposition(), LockDisposition::GovRootExclusive);
    }

    #[test]
    fn test_read_plans_are_lock_free() {
        let status = global(Op::Builtin(BuiltinOp::Status));
        assert_eq!(status.lock_disposition(), LockDisposition::None);

        let plan = plan_get("RFC-0001", Some("title")).expect("get");
        assert!(matches!(plan.scope, Scope::Target { .. }));
        assert!(matches!(plan.op, Op::Get));
        assert_eq!(plan.lock_disposition(), LockDisposition::None);
    }

    #[test]
    fn test_lock_disposition_is_lock_free_for_inspect_commands() {
        assert_eq!(
            global(Op::Builtin(BuiltinOp::Status)).lock_disposition(),
            LockDisposition::None
        );
        assert_eq!(
            plan_get("RFC-0001", Some("title"))
                .expect("get")
                .lock_disposition(),
            LockDisposition::None
        );
        assert_eq!(
            plan_show(
                cmd::edit::ArtifactType::Adr,
                "ADR-0038",
                OutputFormat::Table
            )
            .lock_disposition(),
            LockDisposition::None
        );
    }

    #[test]
    fn test_lock_disposition_requires_lock_for_mutating_commands() {
        assert_eq!(
            global(Op::Builtin(BuiltinOp::Init { force: false })).lock_disposition(),
            LockDisposition::GovRootExclusive
        );
        assert_eq!(
            plan_edit(
                "WI-2026-04-07-004",
                "acceptance_criteria[0]",
                tick_action(OwnedMatchOptions::default(), TickStatus::Done),
                EditExtras::default(),
            )
            .expect("work edit")
            .lock_disposition(),
            LockDisposition::GovRootExclusive
        );
        assert_eq!(
            plan_lifecycle(
                cmd::edit::ArtifactType::WorkItem,
                "WI-2026-04-07-004",
                LifecycleOp::MoveWork {
                    file_or_id: std::path::PathBuf::from("WI-2026-04-07-004"),
                    status: WorkItemStatus::Done,
                },
            )
            .lock_disposition(),
            LockDisposition::GovRootExclusive
        );
    }

    #[test]
    fn test_target_resolves_get_and_edit_to_same_field_target() {
        let get = crate::AdrCommand::Get(crate::CommonGetArgs {
            id: "ADR-0038".to_string(),
            field: Some("alternatives[1].status".to_string()),
        })
        .to_plan()
        .expect("get routed");
        let edit = crate::AdrCommand::Edit(crate::AdrEditArgs {
            common: crate::CommonEditArgs {
                id: "ADR-0038".to_string(),
                path: "alternatives[1].status".to_string(),
                action: EditActionArgs {
                    set: None,
                    add: None,
                    remove: None,
                    tick: Some(TickStatus::Accepted),
                    stdin: false,
                    at: None,
                    exact: false,
                    regex: false,
                    all: false,
                },
            },
            pro: vec![],
            con: vec![],
            reject_reason: None,
        })
        .to_plan()
        .expect("edit routed");

        match ((&get.op, &get.scope), (&edit.op, &edit.scope)) {
            (
                (
                    Op::Get,
                    Scope::Target {
                        artifact: get_artifact,
                        id: get_id,
                        target: get_target,
                    },
                ),
                (
                    Op::Edit(EditOp::Field { .. }),
                    Scope::Target {
                        artifact: edit_artifact,
                        id: edit_id,
                        target: edit_target,
                    },
                ),
            ) => {
                assert_eq!(get_artifact, edit_artifact);
                assert_eq!(get_id, edit_id);
                assert_eq!(get_target, edit_target);
            }
            other => panic!("expected field targets, got {other:?}"),
        }
    }

    #[test]
    fn test_owned_edit_action_rejects_selector_flags_for_set() {
        let err = owned_edit_action(&EditActionArgs {
            set: Some(Some("x".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: Some(0),
            exact: false,
            regex: false,
            all: false,
        })
        .expect_err("set with --at should fail");

        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    }

    #[test]
    fn test_owned_edit_action_rejects_stdin_for_remove() {
        let err = owned_edit_action(&EditActionArgs {
            set: None,
            add: None,
            remove: Some(None),
            tick: None,
            stdin: true,
            at: Some(0),
            exact: false,
            regex: false,
            all: false,
        })
        .expect_err("remove with --stdin should fail");

        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    }

    #[test]
    fn test_from_clause_command_rejects_mixed_canonical_and_legacy_edit_flags() {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: Some("text".to_string()),
            set: Some(Some("Updated".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: Some("legacy".to_string()),
            text_file: None,
        };

        let err = cmd.to_plan().expect_err("mixed modes should fail");
        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    }

    #[test]
    fn test_work_tick_defaults_status_to_done() {
        let cli = crate::Cli::parse_from([
            "govctl",
            "work",
            "tick",
            "WI-2026-04-07-001",
            "acceptance_criteria",
            "Criterion 1",
        ]);

        match cli.command {
            crate::Commands::Work {
                command: crate::WorkCommand::Tick(crate::WorkTickArgs { status, .. }),
            } => assert!(matches!(status, WorkTickStatus::Done)),
            _ => panic!("expected work tick command"),
        }
    }

    #[test]
    fn test_rfc_get_help_restores_resource_specific_examples() {
        let err = match crate::Cli::try_parse_from(["govctl", "rfc", "get", "--help"]) {
            Ok(_) => panic!("help should exit"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        let help = err.to_string();
        assert!(help.contains("VALID FIELDS:"), "help: {help}");
        assert!(
            help.contains("govctl rfc get RFC-0001 title"),
            "help: {help}"
        );
    }

    #[test]
    fn test_work_get_help_restores_resource_specific_examples() {
        let err = match crate::Cli::try_parse_from(["govctl", "work", "get", "--help"]) {
            Ok(_) => panic!("help should exit"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        let help = err.to_string();
        assert!(help.contains("VALID FIELDS:"), "help: {help}");
        assert!(
            help.contains("acceptance_criteria[0].status"),
            "help: {help}"
        );
    }
}
