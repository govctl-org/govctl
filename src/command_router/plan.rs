use super::{OwnedEditAction, execute};
use crate::cmd;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
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
        work_items: Vec<String>,
    },
    LoopList {
        output: crate::OutputFormat,
    },
    LoopShow {
        loop_id: String,
    },
    LoopResume {
        loop_id: Option<String>,
        work_items: Vec<String>,
    },
    LoopReplan {
        loop_id: String,
    },
    LoopAdd {
        loop_id: String,
        work_items: Vec<String>,
    },
    LoopRemove {
        loop_id: String,
        work_items: Vec<String>,
    },
    LoopRun {
        loop_id: Option<String>,
        work_items: Vec<String>,
        max_rounds: u32,
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
        match &self.op {
            Op::Builtin(BuiltinOp::Check { .. })
            | Op::Builtin(BuiltinOp::Status)
            | Op::Builtin(BuiltinOp::Verify { .. })
            | Op::Builtin(BuiltinOp::Describe { .. })
            | Op::Builtin(BuiltinOp::Completions { .. })
            | Op::Builtin(BuiltinOp::SelfUpdate { .. })
            | Op::Builtin(BuiltinOp::TagList { .. })
            | Op::Builtin(BuiltinOp::LoopShow { .. })
            | Op::Builtin(BuiltinOp::LoopResume { .. })
            | Op::Get
            | Op::List { .. }
            | Op::Show { .. } => LockDisposition::None,
            #[cfg(feature = "tui")]
            Op::Builtin(BuiltinOp::Tui) => LockDisposition::None,
            _ => LockDisposition::GovRootExclusive,
        }
    }

    pub fn execute(&self, config: &Config, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
        execute::execute_plan(self, config, op)
    }
}
