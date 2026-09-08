use super::{OwnedEditAction, execute};
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::{DiagnosticResult, Diagnostics};
use crate::model::{ClauseKind, RfcPhase, WorkItemStatus};
use crate::write::{BumpLevel, WriteOp};
use crate::{
    FinalizeStatus, GetOutputFormat, ListOutputFormat, ListTarget, OutputFormat, RenderTarget,
    ShowOutputFormat, TraceOutputFormat,
};
use std::path::PathBuf;

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
    Agent {
        operation: agent_plugin_installer::AgentPluginOperation,
        selector: agent_plugin_installer::AgentSelector,
    },
    AgentHook {
        event: crate::AgentHookEvent,
    },
    Check,
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
    Search {
        query: Vec<String>,
        types: Vec<ListTarget>,
        tags: Vec<String>,
        limit: Option<usize>,
        output: OutputFormat,
        reindex: bool,
    },
    Describe {
        context: bool,
    },
    Completions {
        shell: clap_complete::Shell,
    },
    SelfUpdate {
        check: bool,
    },
    #[cfg(feature = "tui")]
    Tui,
    ReleaseCut {
        version: String,
        date: Option<String>,
    },
    ReleaseUndo {
        expected_version: String,
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
    LoopStart {
        loop_id: Option<String>,
        work_ids: Vec<String>,
    },
    LoopList {
        filter: Option<String>,
        limit: Option<usize>,
        output: crate::OutputFormat,
    },
    LoopShow {
        loop_id: String,
    },
    LoopResume {
        loop_id: String,
    },
    LoopReplan {
        loop_id: String,
    },
    LoopAdd {
        loop_id: String,
        field: String,
        value: String,
    },
    LoopRemove {
        loop_id: String,
        field: String,
        value: String,
    },
    LoopRun {
        loop_id: String,
        target_work_ids: Vec<String>,
    },
    ConformanceList {
        filter: Option<String>,
        limit: Option<usize>,
        output: Option<ListOutputFormat>,
        tags: Vec<String>,
    },
    ConformanceGet {
        id: String,
        field: Option<String>,
        output: Option<GetOutputFormat>,
    },
    ConformanceShow {
        id: String,
        output: ShowOutputFormat,
        history: bool,
    },
    ConformanceNew {
        title: String,
        path: String,
        selector: String,
        requirements: Vec<String>,
        guards: Vec<String>,
        id: Option<String>,
    },
    ConformanceEdit {
        id: String,
        path: String,
        action: OwnedEditAction,
    },
    ConformanceDelete {
        id: String,
        force: bool,
    },
    ConformanceTrace {
        target: Option<String>,
        output: Option<TraceOutputFormat>,
    },
    /// List artifact claims — [[RFC-0010:C-ARTIFACT-CLAIM]].
    ClaimList,
    /// Release a claim held by this workspace.
    ClaimRelease {
        id: String,
    },
    /// Take over a claim held by another workspace (audited).
    ClaimSteal {
        id: String,
    },
}

impl BuiltinOp {
    fn is_lock_free(&self) -> bool {
        match self {
            Self::Check { .. }
            | Self::Status
            | Self::Verify { .. }
            | Self::Describe { .. }
            | Self::Completions { .. }
            | Self::SelfUpdate { .. }
            // [[RFC-0002:C-AGENT-INTEGRATION]]: agent operations manage
            // user-scoped runtime integration, not governed project state.
            | Self::Agent { .. }
            | Self::AgentHook { .. }
            | Self::TagList { .. }
            | Self::LoopList { .. }
            | Self::LoopShow { .. }
            | Self::LoopResume { .. } => true,
            Self::ConformanceList { .. }
            | Self::ConformanceGet { .. }
            | Self::ConformanceShow { .. }
            | Self::ConformanceTrace { .. } => true,
            // [[RFC-0010:C-ARTIFACT-CLAIM]]: claim commands mutate only
            // coordination state in shared VCS storage (serialized by the
            // registry allocation lock), never the gov tree, so they stay
            // outside the gov-root write-lock class.
            Self::ClaimList | Self::ClaimRelease { .. } | Self::ClaimSteal { .. } => true,
            // [[RFC-0002:C-SEARCH-COMMAND]]: search may sync `.govctl/`
            // derived local state but must not mutate governed artifacts or
            // rendered docs; [[RFC-0004:C-DEFINITIONS]] keeps that outside the
            // gov-root write-lock class.
            Self::Search { .. } => true,
            #[cfg(feature = "tui")]
            Self::Tui => true,
            _ => false,
        }
    }
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
    Field { action: OwnedEditAction },
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
    AcceptAdr {
        force: bool,
    },
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
        output: Option<ListOutputFormat>,
        /// Tags to filter by (artifact must have ALL specified tags) — [[RFC-0002:C-CRUD-VERBS]]
        tags: Vec<String>,
    },
    Get {
        output: Option<GetOutputFormat>,
    },
    Show {
        output: ShowOutputFormat,
        history: bool,
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

impl Op {
    fn is_lock_free(&self) -> bool {
        match self {
            Self::Builtin(builtin) => builtin.is_lock_free(),
            Self::Get { .. } | Self::List { .. } | Self::Show { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockDisposition {
    None,
    GovRootExclusive,
}

/// Command scope for multi-workspace coordination — [[RFC-0010:C-COMMAND-SCOPE]].
///
/// Every command has exactly one scope. Trunk-scoped commands (`release`,
/// `migrate`) run only in the primary workspace; branch-content commands
/// mutate governed artifacts and run in any workspace; workspace-scoped
/// commands are read-only or local-execution and run anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    Workspace,
    BranchContent,
    Trunk,
}

#[derive(Debug, Clone)]
pub struct CommandPlan {
    pub scope: Scope,
    pub op: Op,
}

impl CommandPlan {
    pub(crate) fn new(scope: Scope, op: Op) -> Self {
        Self { scope, op }
    }

    pub fn lock_disposition(&self) -> LockDisposition {
        if self.op.is_lock_free() {
            LockDisposition::None
        } else {
            LockDisposition::GovRootExclusive
        }
    }

    /// Classify the command per [[RFC-0010:C-COMMAND-SCOPE]].
    pub fn command_scope(&self) -> CommandScope {
        match &self.op {
            Op::Builtin(
                BuiltinOp::Migrate | BuiltinOp::ReleaseCut { .. } | BuiltinOp::ReleaseUndo { .. },
            ) => CommandScope::Trunk,
            // [[RFC-0010:C-COMMAND-SCOPE]]: read-only and local-execution
            // commands are workspace scoped. `claim list` only reads
            // coordination state, and loop commands manage local execution
            // state under `.govctl/`, never governed artifacts.
            Op::Builtin(
                BuiltinOp::ClaimList
                | BuiltinOp::LoopStart { .. }
                | BuiltinOp::LoopList { .. }
                | BuiltinOp::LoopShow { .. }
                | BuiltinOp::LoopResume { .. }
                | BuiltinOp::LoopReplan { .. }
                | BuiltinOp::LoopAdd { .. }
                | BuiltinOp::LoopRemove { .. }
                | BuiltinOp::LoopRun { .. },
            ) => CommandScope::Workspace,
            // [[RFC-0010:C-COMMAND-SCOPE]]: claim release and steal mutate
            // coordination state only; they are branch-content scoped for
            // enforcement purposes but stay lock-free (see
            // `BuiltinOp::is_lock_free`), never acquiring the gov-root write
            // lock.
            Op::Builtin(BuiltinOp::ClaimRelease { .. } | BuiltinOp::ClaimSteal { .. }) => {
                CommandScope::BranchContent
            }
            _ if matches!(self.lock_disposition(), LockDisposition::GovRootExclusive) => {
                CommandScope::BranchContent
            }
            _ => CommandScope::Workspace,
        }
    }

    pub fn execute(&self, config: &Config, op: WriteOp) -> DiagnosticResult<Diagnostics> {
        execute::execute_plan(self, config, op)
    }
}
