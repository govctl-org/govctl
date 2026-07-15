use super::{OwnedEditAction, execute};
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::{DiagnosticResult, Diagnostics};
use crate::model::{ChangelogCategory, ClauseKind, RfcPhase, WorkItemStatus};
use crate::write::{BumpLevel, WriteOp};
use crate::{FinalizeStatus, ListTarget, OutputFormat, RenderTarget};
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
            | Self::TagList { .. }
            | Self::LoopList { .. }
            | Self::LoopShow { .. }
            | Self::LoopResume { .. } => true,
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

impl Op {
    fn is_lock_free(&self) -> bool {
        match self {
            Self::Builtin(builtin) => builtin.is_lock_free(),
            Self::Get | Self::List { .. } | Self::Show { .. } => true,
            _ => false,
        }
    }
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
    pub(super) fn new(scope: Scope, op: Op) -> Self {
        Self { scope, op }
    }

    pub fn lock_disposition(&self) -> LockDisposition {
        if self.op.is_lock_free() {
            LockDisposition::None
        } else {
            LockDisposition::GovRootExclusive
        }
    }

    pub fn execute(&self, config: &Config, op: WriteOp) -> DiagnosticResult<Diagnostics> {
        execute::execute_plan(self, config, op)
    }
}
