//! Shared coordination registry and cross-workspace ID reservation.
//!
//! Implements [[RFC-0010:C-REGISTRY]], [[RFC-0010:C-ID-RESERVATION]],
//! [[RFC-0010:C-ARTIFACT-CLAIM]], and [[RFC-0010:C-PRESENCE]].
//!
//! The registry lives in the clone's shared VCS storage (git common dir or
//! jj repo dir), namespaced by a short hash of the repo-relative governed
//! root so that a monorepo hosting several `gov/` roots keeps independent
//! registries while every workspace of one governed project shares a single
//! registry. Each reservation, claim, and audit event is one record file;
//! a single flock-based lock file serializes mutation across processes and
//! workspaces, while read sessions stay lock-free so read-only commands
//! never block — [[RFC-0010:C-REGISTRY]].
//!
//! Degradation follows the caution rule of [[RFC-0010:C-REGISTRY]]: without
//! version control the registry is inapplicable and sessions stay silent;
//! with version control present, any registry failure or corruption degrades
//! the session to single-checkout behavior with a warning — artifact
//! creation itself never fails because of the registry.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::workspace::{self, StorageLookup, VcsKind};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Lock file serializing allocation across the clone's workspaces.
const LOCK_FILE_NAME: &str = "allocation.lock";
/// Directory holding one record file per reservation.
const RESERVATIONS_DIR: &str = "reservations";
/// Directory holding one record file per artifact claim.
const CLAIMS_DIR: &str = "claims";
/// Directory holding one record file per work-item presence record.
const PRESENCE_DIR: &str = "presence";
/// Directory holding append-only audit events; never pruned, so claim
/// takeovers stay auditable after the transferred claim expires.
const AUDIT_DIR: &str = "audit";
/// Directory holding undecodable records moved out of the active
/// directories. Quarantine preserves the record for forensics while keeping
/// later invocations from tripping over it again; it is never pruned
/// automatically and records here are never deleted by govctl.
const CORRUPT_DIR: &str = "corrupt";
/// Backoff between try_lock attempts (mirrors src/lock.rs).
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One ID reservation: the owning workspace and the artifact file whose
/// existence keeps the reservation live.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Reservation {
    id: String,
    /// Owning workspace root (informational; identifies the claimant).
    workspace: PathBuf,
    /// Absolute artifact path; the reservation is live while this exists.
    artifact: PathBuf,
}

/// One artifact claim: an exclusive record binding an RFC to the workspace
/// performing version-semantics operations on it —
/// [[RFC-0010:C-ARTIFACT-CLAIM]].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claim {
    /// Claimed RFC ID.
    pub id: String,
    /// Canonical root of the owning workspace.
    pub workspace: PathBuf,
    /// Owning workspace's last activity, seconds since the Unix epoch.
    pub last_activity: u64,
}

/// One work-item presence record: an advisory note that a workspace is
/// actively working on the work item — [[RFC-0010:C-PRESENCE]]. The title is
/// recorded so other workspaces can identify the item even before it reaches
/// shared version-control history. Presence follows the claim liveness rules
/// and never blocks any operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Presence {
    /// Active work item ID.
    pub id: String,
    /// Work item title at registration time (informational).
    pub title: String,
    /// Canonical root of the owning workspace.
    pub workspace: PathBuf,
    /// Owning workspace's last activity, seconds since the Unix epoch.
    pub last_activity: u64,
}

/// Append-only audit record of a claim takeover; names both workspaces.
/// Audit events are never expired or pruned.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEvent {
    /// Event kind; currently always `takeover`.
    event: String,
    /// RFC whose claim was transferred.
    id: String,
    from_workspace: PathBuf,
    to_workspace: PathBuf,
    /// Seconds since the Unix epoch.
    timestamp: u64,
}

/// A registry allocation session, holding the allocation lock for the
/// lifetime of one artifact-creating command.
///
/// All operations are non-fatal: a session with `inner == None` is either
/// silently inactive (no VCS) or degraded (warning recorded in `warnings`).
pub struct ReservationSession {
    inner: Option<Active>,
    warnings: Vec<Diagnostic>,
}

struct Active {
    /// `<storage>/govctl/registry/<hash>/` for this governed project.
    root: PathBuf,
    /// Canonical gov root relative to the workspace root; the history pathspec.
    gov_rel: PathBuf,
    /// Canonical root of the invoking workspace.
    repo_root: PathBuf,
    kind: VcsKind,
    /// Lazily scanned IDs recorded in shared version-control history.
    history: Option<Vec<String>>,
    /// Allocation lock held for the session's lifetime (write sessions only).
    _lock: Option<std::fs::File>,
}

/// An opened registry: shared storage located and the registry directory
/// resolved. Write sessions (`lock` held) create the registry directory and
/// serialize allocation across processes; read sessions (`lock: None`) are
/// lock-free so read-only commands never block behind a concurrent writer —
/// [[RFC-0010:C-REGISTRY]].
struct OpenedRegistry {
    /// `<storage>/govctl/registry/<hash>/` for this governed project.
    root: PathBuf,
    /// Canonical gov root relative to the workspace root; the history pathspec.
    gov_rel: PathBuf,
    /// Canonical root of the invoking workspace.
    repo_root: PathBuf,
    kind: VcsKind,
    /// Allocation lock; held only by sessions that mutate the registry.
    lock: Option<std::fs::File>,
}

/// Outcome of opening the coordination registry.
enum OpenOutcome {
    /// No version control, or a dry-run preview: coordination is silently
    /// inactive per [[RFC-0010:C-REGISTRY]].
    Inactive,
    Ready(OpenedRegistry),
    /// Version control is present but the registry cannot be used; callers
    /// degrade to single-checkout behavior with a warning.
    Degraded(String),
}

/// Open the registry for `config`, acquiring the allocation lock.
fn open_registry(config: &Config, preview: bool) -> OpenOutcome {
    open_registry_with(config, preview, activate)
}

/// Open the registry read-only: no allocation lock, no registry creation, no
/// activity refresh. [[RFC-0010:C-REGISTRY]] requires that registry access
/// never block read-only commands, so this path never touches the lock and
/// never writes to shared storage.
fn open_registry_readonly(config: &Config, preview: bool) -> OpenOutcome {
    open_registry_with(config, preview, activate_readonly)
}

fn open_registry_with(
    config: &Config,
    preview: bool,
    activate: fn(&Config, &workspace::SharedStorage) -> Result<OpenedRegistry, String>,
) -> OpenOutcome {
    if preview {
        return OpenOutcome::Inactive;
    }
    let storage = match workspace::shared_storage(&config.gov_root) {
        StorageLookup::NoVcs => return OpenOutcome::Inactive,
        StorageLookup::Found(storage) => storage,
        StorageLookup::Unavailable => {
            return OpenOutcome::Degraded(
                "version control is present but its shared storage cannot be determined"
                    .to_string(),
            );
        }
    };
    match activate(config, &storage) {
        Ok(opened) => OpenOutcome::Ready(opened),
        Err(reason) => OpenOutcome::Degraded(reason),
    }
}

/// Resolve the registry namespace for this governed project.
fn registry_paths(
    config: &Config,
    storage: &workspace::SharedStorage,
) -> Result<(PathBuf, PathBuf), String> {
    let gov_root = std::fs::canonicalize(&config.gov_root)
        .map_err(|err| format!("cannot canonicalize the gov root: {err}"))?;
    let gov_rel = gov_root
        .strip_prefix(&storage.repo_root)
        .map_err(|_| "gov root is not inside the workspace root".to_string())?
        .to_path_buf();
    let root = registry_root(&storage.storage, &gov_rel);
    Ok((gov_rel, root))
}

/// Open the registry for reading only. A missing registry directory reads as
/// empty; nothing is created and no lock is acquired.
fn activate_readonly(
    config: &Config,
    storage: &workspace::SharedStorage,
) -> Result<OpenedRegistry, String> {
    let (gov_rel, root) = registry_paths(config, storage)?;
    Ok(OpenedRegistry {
        root,
        gov_rel,
        repo_root: storage.repo_root.clone(),
        kind: storage.kind,
        lock: None,
    })
}

fn activate(config: &Config, storage: &workspace::SharedStorage) -> Result<OpenedRegistry, String> {
    let (gov_rel, root) = registry_paths(config, storage)?;
    let reservations = root.join(RESERVATIONS_DIR);
    std::fs::create_dir_all(&reservations)
        .map_err(|err| format!("cannot create the registry at {}: {err}", root.display()))?;
    let lock_path = root.join(LOCK_FILE_NAME);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|err| {
            format!(
                "cannot open the registry lock at {}: {err}",
                lock_path.display()
            )
        })?;
    let deadline = Instant::now() + Duration::from_secs(config.concurrency.lock_timeout_secs);
    loop {
        match lock.try_lock_exclusive() {
            Ok(()) => break,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err("timed out waiting for the registry allocation lock".to_string());
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(err) => return Err(format!("cannot lock the registry: {err}")),
        }
    }
    // [[RFC-0010:C-ARTIFACT-CLAIM]] and [[RFC-0010:C-PRESENCE]]: touching the
    // registry refreshes the last-activity timestamps of claims and presence
    // records held by this workspace. Only write sessions refresh, so
    // read-only commands never become writers.
    refresh_workspace_claims(&root, &storage.repo_root);
    refresh_workspace_presence(&root, &storage.repo_root);
    Ok(OpenedRegistry {
        root,
        gov_rel,
        repo_root: storage.repo_root.clone(),
        kind: storage.kind,
        lock: Some(lock),
    })
}

impl ReservationSession {
    /// Open a session for `config`, acquiring the allocation lock.
    ///
    /// Dry-run previews never touch the registry. Without version control
    /// the session is silently inactive. With version control present, any
    /// failure degrades the session and records a warning.
    pub fn begin(config: &Config, preview: bool) -> Self {
        let mut session = Self {
            inner: None,
            warnings: vec![],
        };
        match open_registry(config, preview) {
            OpenOutcome::Inactive => {}
            OpenOutcome::Degraded(reason) => session.degrade(reason, &config.gov_root),
            OpenOutcome::Ready(opened) => {
                session.inner = Some(Active {
                    root: opened.root,
                    gov_rel: opened.gov_rel,
                    repo_root: opened.repo_root,
                    kind: opened.kind,
                    history: None,
                    _lock: opened.lock,
                });
            }
        }
        session
    }

    fn degrade(&mut self, reason: String, file: &Path) {
        self.inner = None;
        self.warnings.push(Diagnostic::new(
            DiagnosticCode::W0115RegistryDegraded,
            format!(
                "Coordination registry unavailable ({reason}); falling back to \
                 single-checkout ID allocation. Uniqueness across workspaces of \
                 this clone is not guaranteed until the registry is repopulated."
            ),
            file.display().to_string(),
        ));
    }

    /// Maximum sequence witnessed for `prefix`, combining the local gov tree
    /// maximum with live registry reservations and shared version-control
    /// history per [[RFC-0010:C-ID-RESERVATION]]. On registry failure the
    /// session degrades with a warning and the local maximum is returned.
    pub fn max_witnessed(&mut self, prefix: &str, local_max: u32) -> u32 {
        if self.inner.is_none() {
            return local_max;
        }
        match self.witnessed_max(prefix) {
            Ok(max) => max.max(local_max),
            Err(reason) => {
                let file = self
                    .inner
                    .as_ref()
                    .map(|active| active.root.clone())
                    .unwrap_or_default();
                self.degrade(reason, &file);
                local_max
            }
        }
    }

    fn witnessed_max(&mut self, prefix: &str) -> Result<u32, String> {
        let mut max = 0u32;
        for reservation in self.live_reservations()? {
            if let Some(seq) = sequence_of(&reservation.id, prefix) {
                max = max.max(seq);
            }
        }
        for id in self.history_ids()? {
            if let Some(seq) = sequence_of(id, prefix) {
                max = max.max(seq);
            }
        }
        Ok(max)
    }

    /// Record `id` as reserved, owned by this workspace, with `artifact` as
    /// the file whose existence keeps the reservation live. The session's
    /// allocation lock makes this atomic with allocation.
    pub fn record(&mut self, id: &str, artifact: &Path) {
        let Some(active) = &self.inner else {
            return;
        };
        let reservation = Reservation {
            id: id.to_string(),
            workspace: active.repo_root.clone(),
            artifact: std::fs::canonicalize(artifact).unwrap_or_else(|_| artifact.to_path_buf()),
        };
        if let Err(reason) = write_reservation(&active.root, &reservation) {
            self.warnings.push(Diagnostic::new(
                DiagnosticCode::W0115RegistryDegraded,
                format!(
                    "Could not record ID reservation ({reason}); the reservation is \
                     lost but the artifact was created. Uniqueness across workspaces \
                     of this clone is not guaranteed until the registry is repopulated."
                ),
                active.root.display().to_string(),
            ));
        }
    }

    /// Warnings collected by this session.
    pub fn into_warnings(self) -> Vec<Diagnostic> {
        self.warnings
    }

    /// Live reservations. A reservation whose artifact file still exists is
    /// live. Otherwise it is pruned only when its ID is witnessed in shared
    /// version-control history — [[RFC-0010:C-ID-RESERVATION]] permits
    /// discarding a reservation only once the artifact is recorded there.
    /// An unwitnessed record (uncommitted deletion, or a deleted workspace
    /// holding unmerged work) stays reserved so the ID cannot be reallocated.
    /// An undecodable record is quarantined aside and skipped with a warning,
    /// so one corrupt file cannot degrade every later allocation —
    /// [[RFC-0010:C-REGISTRY]] limits corruption to losing the uniqueness
    /// guarantee for the affected record.
    fn live_reservations(&mut self) -> Result<Vec<Reservation>, String> {
        let dir = {
            let Some(active) = self.inner.as_ref() else {
                return Ok(vec![]);
            };
            active.root.join(RESERVATIONS_DIR)
        };
        let entries = std::fs::read_dir(&dir)
            .map_err(|err| format!("cannot read the registry at {}: {err}", dir.display()))?;
        let mut records = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| format!("cannot read a registry entry: {err}"))?;
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "toml") {
                continue;
            }
            match read_reservation(&path) {
                Ok(record) => records.push((path, record)),
                Err(reason) => self.quarantine_corrupt_reservation(&path, reason)?,
            }
        }
        let witnessed = self.history_ids()?;
        let mut live = Vec::new();
        for (path, record) in records {
            if record.artifact.exists() || !witnessed.contains(&record.id) {
                live.push(record);
            } else {
                // Witnessed in shared history: the reservation is dead
                // weight. Failure to prune is conservative — the record
                // merely stays reserved until a later cleanup.
                let _ = std::fs::remove_file(&path);
            }
        }
        Ok(live)
    }

    /// Move an undecodable reservation record into the registry's `corrupt/`
    /// directory — never delete it — and warn. After quarantine the record
    /// no longer poisons later allocations; the uniqueness guarantee for its
    /// ID is knowingly weakened, per the degradation rule of
    /// [[RFC-0010:C-REGISTRY]].
    fn quarantine_corrupt_reservation(
        &mut self,
        path: &Path,
        reason: String,
    ) -> Result<(), String> {
        let root = self
            .inner
            .as_ref()
            .map(|active| active.root.clone())
            .unwrap_or_default();
        let target = quarantine_path(&root, path)?;
        std::fs::rename(path, &target).map_err(|err| {
            format!(
                "cannot quarantine the corrupt reservation at {}: {err}",
                path.display()
            )
        })?;
        self.warnings.push(Diagnostic::new(
            DiagnosticCode::W0115RegistryDegraded,
            format!(
                "Detected a corrupt reservation record ({reason}); it was moved aside \
                 to {} and skipped. Uniqueness of its ID across workspaces of this \
                 clone is not guaranteed until the registry is repopulated.",
                target.display()
            ),
            path.display().to_string(),
        ));
        Ok(())
    }

    /// Artifact IDs recorded in the clone's shared version-control history.
    ///
    /// Git: paths and `id = "..."` lines of every gov-tree file ever added,
    /// across all refs — worktree branches live in shared storage, so this
    /// witnesses unmerged committed work too. Colocated jj+git repos share
    /// one store, so the git query applies there as well. Non-colocated jj
    /// has no equivalent cheap query wired up yet; reservations and the
    /// local tree still apply, and pruning stays conservative.
    fn history_ids(&mut self) -> Result<&[String], String> {
        let Some(active) = self.inner.as_mut() else {
            return Ok(&[]);
        };
        if active.history.is_none() {
            let ids = match active.kind {
                VcsKind::Git => git_history_ids(&active.repo_root, &active.gov_rel)?,
                VcsKind::Jj if active.repo_root.join(".git").exists() => {
                    git_history_ids(&active.repo_root, &active.gov_rel)?
                }
                VcsKind::Jj => vec![],
            };
            active.history = Some(ids);
        }
        Ok(active.history.as_deref().unwrap_or(&[]))
    }
}

/// Read-only view of a claim for `govctl claim list`.
#[derive(Debug, Clone)]
pub struct ClaimInfo {
    pub id: String,
    pub workspace: PathBuf,
    /// Seconds since the Unix epoch of the owning workspace's last activity.
    pub last_activity: u64,
    /// Whether the claim currently blocks other workspaces.
    pub live: bool,
}

/// Outcome of a claim release attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseOutcome {
    /// The claim record was removed.
    Released,
    /// No claim exists for the RFC.
    NotHeld,
}

/// Outcome of a claim takeover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StealOutcome {
    /// The claim transferred from another workspace (audited) or an expired
    /// or absent claim was acquired.
    Taken,
    /// The invoking workspace already holds a live claim.
    AlreadyHeld,
}

/// Artifact-claim operations on the coordination registry —
/// [[RFC-0010:C-ARTIFACT-CLAIM]].
///
/// Like the reservation session, all coordination is non-fatal for governed
/// artifacts: a session with `inner == None` is silently inactive (no VCS,
/// or a dry-run preview) or degraded (warning recorded), and claim exclusion
/// then reduces to single-checkout behavior per [[RFC-0010:C-REGISTRY]].
pub struct ClaimSession {
    inner: Option<OpenedRegistry>,
    ttl: Duration,
    warnings: Vec<Diagnostic>,
    /// Claims newly acquired by this session (created or taken over from an
    /// expired or foreign record), so a failed multi-RFC operation can
    /// release them before returning its error.
    acquired: Vec<String>,
}

impl ClaimSession {
    /// Open a claim session for `config`. The inactivity period comes from
    /// `[workspace] claim_ttl_days` (default 7).
    pub fn begin(config: &Config, preview: bool) -> Self {
        Self::open(config, preview, open_registry)
    }

    /// Open a lock-free read-only claim session — [[RFC-0010:C-REGISTRY]]
    /// forbids blocking read-only commands on the registry lock.
    pub fn begin_readonly(config: &Config) -> Self {
        Self::open(config, false, open_registry_readonly)
    }

    fn open(config: &Config, preview: bool, open: fn(&Config, bool) -> OpenOutcome) -> Self {
        let mut session = Self {
            inner: None,
            ttl: claim_ttl(config),
            warnings: vec![],
            acquired: vec![],
        };
        match open(config, preview) {
            OpenOutcome::Inactive => {}
            OpenOutcome::Degraded(reason) => session.degrade(reason, &config.gov_root),
            OpenOutcome::Ready(opened) => session.inner = Some(opened),
        }
        session
    }

    /// Whether the registry is open (claims enforceable).
    pub fn is_active(&self) -> bool {
        self.inner.is_some()
    }

    /// Whether opening the registry degraded with a warning.
    pub fn is_degraded(&self) -> bool {
        !self.warnings.is_empty()
    }

    fn degrade(&mut self, reason: String, file: &Path) {
        self.inner = None;
        self.warnings.push(Diagnostic::new(
            DiagnosticCode::W0115RegistryDegraded,
            format!(
                "Coordination registry unavailable ({reason}); artifact claims are \
                 inactive. Version-semantics operations proceed without \
                 cross-workspace exclusion until the registry is repopulated."
            ),
            file.display().to_string(),
        ));
    }

    /// Acquire an exclusive claim on every RFC in `ids` —
    /// [[RFC-0010:C-ARTIFACT-CLAIM]]. A live claim held by another workspace
    /// fails the command before any mutation with a diagnostic naming the
    /// claiming workspace; claims this invocation already acquired are
    /// released first, so a failed multi-RFC acquisition (e.g. supersede)
    /// leaves no claim behind. Claims persist as expiring ownership: they are
    /// refreshed by activity in the owning workspace and released
    /// explicitly, by takeover, or by expiry.
    pub fn acquire(&mut self, ids: &[&str]) -> DiagnosticResult<()> {
        let Some((root, repo_root)) = self.handle() else {
            return Ok(());
        };
        let now = now_secs();
        for id in ids {
            let path = claim_path(&root, id);
            let existing = match read_claim(&path) {
                Ok(existing) => existing,
                Err(reason) => {
                    self.degrade(reason, &path);
                    return Ok(());
                }
            };
            if let Some(claim) = &existing
                && claim.workspace != repo_root
                && claim_is_live(claim, self.ttl, now)
            {
                self.release_acquired();
                return Err(claim_conflict(id, claim, self.ttl));
            }
            // Free, expired, or already ours: (re)acquire and refresh.
            let claim = Claim {
                id: (*id).to_string(),
                workspace: repo_root.clone(),
                last_activity: now,
            };
            if let Err(reason) = write_claim(&root, &claim) {
                self.degrade(reason, &path);
                return Ok(());
            }
            // Track claims this invocation newly created or took over, so a
            // failing operation can release them. A refresh of a claim this
            // workspace already held is not tracked: it predates the
            // invocation and must survive a rollback.
            if existing.is_none_or(|prior| prior.workspace != repo_root) {
                self.acquired.push((*id).to_string());
            }
        }
        Ok(())
    }

    /// Release claims newly acquired by this session, best-effort. Used to
    /// roll back an operation that fails after acquiring its claims: the
    /// invocation did no version-semantics work, so its claims must not stay
    /// behind. Claims this workspace held before the session are untouched.
    pub fn release_acquired(&mut self) {
        let Some((root, repo_root)) = self.handle() else {
            self.acquired.clear();
            return;
        };
        for id in std::mem::take(&mut self.acquired) {
            let path = claim_path(&root, &id);
            // Remove only a record this workspace still owns.
            if matches!(read_claim(&path), Ok(Some(claim)) if claim.workspace == repo_root) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    /// Non-blocking warnings for content edits on RFCs with a live claim
    /// held by another workspace — [[RFC-0010:C-ARTIFACT-CLAIM]].
    pub fn foreign_claim_warnings(&mut self, ids: &[&str]) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let Some((root, repo_root)) = self.handle() else {
            return diags;
        };
        let now = now_secs();
        for id in ids {
            let path = claim_path(&root, id);
            match read_claim(&path) {
                Ok(Some(claim))
                    if claim.workspace != repo_root && claim_is_live(&claim, self.ttl, now) =>
                {
                    diags.push(Diagnostic::new(
                        DiagnosticCode::W0116ArtifactClaimHeld,
                        format!(
                            "{id} is claimed for version-semantics work by workspace {}. \
                             Content edits are not blocked, but coordinate before lifecycle \
                             operations on {id}.",
                            claim.workspace.display()
                        ),
                        (*id).to_string(),
                    ));
                }
                Ok(_) => {}
                Err(reason) => {
                    self.degrade(reason, &path);
                    break;
                }
            }
        }
        diags
    }

    /// Record a corruption warning for an undecodable claim record; the
    /// read-only listing skips the record and continues with the remaining
    /// ones — [[RFC-0010:C-REGISTRY]].
    fn warn_corrupt(&mut self, reason: String, path: &Path) {
        self.warnings.push(Diagnostic::new(
            DiagnosticCode::W0115RegistryDegraded,
            format!(
                "Detected a corrupt claim record ({reason}); the record is ignored. \
                 Cross-workspace claim visibility is degraded until the registry is \
                 repopulated."
            ),
            path.display().to_string(),
        ));
    }

    /// All claims, live and expired, for `govctl claim list`. Undecodable
    /// records are skipped with a corruption warning —
    /// [[RFC-0010:C-REGISTRY]].
    pub fn list(&mut self) -> Vec<ClaimInfo> {
        let Some((root, _)) = self.handle() else {
            return vec![];
        };
        let dir = root.join(CLAIMS_DIR);
        let now = now_secs();
        let mut claims = Vec::new();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return claims;
        };
        for path in entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        {
            match read_claim(&path) {
                Ok(Some(claim)) => claims.push(ClaimInfo {
                    live: claim_is_live(&claim, self.ttl, now),
                    id: claim.id,
                    workspace: claim.workspace,
                    last_activity: claim.last_activity,
                }),
                Ok(None) => {}
                Err(reason) => self.warn_corrupt(reason, &path),
            }
        }
        claims.sort_by(|left, right| left.id.cmp(&right.id));
        claims
    }

    /// Explicitly release a claim — [[RFC-0010:C-ARTIFACT-CLAIM]]. A live
    /// claim held by another workspace cannot be released (that bypass would
    /// defeat the audited takeover); the diagnostic points at `claim steal`.
    pub fn release(&mut self, id: &str) -> DiagnosticResult<ReleaseOutcome> {
        let Some((root, repo_root)) = self.handle() else {
            return Ok(ReleaseOutcome::NotHeld);
        };
        let path = claim_path(&root, id);
        let existing = match read_claim(&path) {
            Ok(existing) => existing,
            Err(reason) => {
                self.degrade(reason, &path);
                return Ok(ReleaseOutcome::NotHeld);
            }
        };
        let Some(claim) = existing else {
            return Ok(ReleaseOutcome::NotHeld);
        };
        if claim.workspace != repo_root && claim_is_live(&claim, self.ttl, now_secs()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0824ArtifactClaimed,
                format!(
                    "Cannot release the claim on {id}: it is held by workspace {}. \
                     Take it over with `govctl claim steal {id}` instead.",
                    claim.workspace.display()
                ),
                id.to_string(),
            ));
        }
        std::fs::remove_file(&path).map_err(|err| {
            Diagnostic::io_error("release claim", err, path.display().to_string())
        })?;
        Ok(ReleaseOutcome::Released)
    }

    /// Take over a claim — [[RFC-0010:C-ARTIFACT-CLAIM]]. Taking over a
    /// claim recorded under another workspace (live or expired) appends an
    /// audit event naming both workspaces; audit events are never pruned.
    /// The claim record is written before its audit event, so a failed write
    /// leaves no audit entry for a transfer that never happened.
    pub fn steal(&mut self, id: &str) -> DiagnosticResult<StealOutcome> {
        let Some((root, repo_root)) = self.handle() else {
            return Ok(StealOutcome::Taken);
        };
        let path = claim_path(&root, id);
        let existing = match read_claim(&path) {
            Ok(existing) => existing,
            Err(reason) => {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    reason,
                    path.display().to_string(),
                ));
            }
        };
        let prior_foreign = match &existing {
            Some(claim)
                if claim.workspace == repo_root && claim_is_live(claim, self.ttl, now_secs()) =>
            {
                return Ok(StealOutcome::AlreadyHeld);
            }
            Some(claim) if claim.workspace != repo_root => Some(claim.workspace.clone()),
            _ => None,
        };
        let claim = Claim {
            id: id.to_string(),
            workspace: repo_root.clone(),
            last_activity: now_secs(),
        };
        write_claim(&root, &claim).map_err(|reason| {
            Diagnostic::new(DiagnosticCode::E0901IoError, reason, id.to_string())
        })?;
        // The transfer happened; only now record it in the audit trail.
        if let Some(from_workspace) = prior_foreign {
            let event = AuditEvent {
                event: "takeover".to_string(),
                id: id.to_string(),
                from_workspace,
                to_workspace: repo_root.clone(),
                timestamp: now_secs(),
            };
            write_audit_event(&root, &event).map_err(|reason| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    reason,
                    root.display().to_string(),
                )
            })?;
        }
        Ok(StealOutcome::Taken)
    }

    /// Warnings collected by this session.
    pub fn into_warnings(self) -> Vec<Diagnostic> {
        self.warnings
    }

    fn handle(&self) -> Option<(PathBuf, PathBuf)> {
        self.inner
            .as_ref()
            .map(|opened| (opened.root.clone(), opened.repo_root.clone()))
    }
}

/// Work-item presence operations on the coordination registry —
/// [[RFC-0010:C-PRESENCE]].
///
/// Presence is advisory visibility, not exclusion: registering or removing a
/// record never fails the invoking command. A session with `inner == None` is
/// silently inactive (no VCS, or a dry-run preview) or degraded (warning
/// recorded) per [[RFC-0010:C-REGISTRY]].
pub struct PresenceSession {
    inner: Option<OpenedRegistry>,
    ttl: Duration,
    warnings: Vec<Diagnostic>,
}

impl PresenceSession {
    /// Open a presence session for `config`. The inactivity period is the
    /// same `[workspace] claim_ttl_days` (default 7) that governs claims.
    pub fn begin(config: &Config, preview: bool) -> Self {
        Self::open(config, preview, open_registry)
    }

    /// Open a lock-free read-only presence session — [[RFC-0010:C-REGISTRY]]
    /// forbids blocking read-only commands on the registry lock. Read
    /// sessions also skip the activity refresh, so they never become writers.
    pub fn begin_readonly(config: &Config) -> Self {
        Self::open(config, false, open_registry_readonly)
    }

    fn open(config: &Config, preview: bool, open: fn(&Config, bool) -> OpenOutcome) -> Self {
        let mut session = Self {
            inner: None,
            ttl: claim_ttl(config),
            warnings: vec![],
        };
        match open(config, preview) {
            OpenOutcome::Inactive => {}
            OpenOutcome::Degraded(reason) => session.degrade(reason, &config.gov_root),
            OpenOutcome::Ready(opened) => session.inner = Some(opened),
        }
        session
    }

    fn degrade(&mut self, reason: String, file: &Path) {
        self.inner = None;
        self.warnings.push(Diagnostic::new(
            DiagnosticCode::W0115RegistryDegraded,
            format!(
                "Coordination registry unavailable ({reason}); work-item presence is \
                 inactive. Other workspaces of this clone will not see work items \
                 activated here until the registry is repopulated."
            ),
            file.display().to_string(),
        ));
    }

    /// Register this workspace as actively working on the work item —
    /// [[RFC-0010:C-PRESENCE]]. Advisory: an existing record (from any
    /// workspace) is overwritten, never a conflict.
    pub fn register(&mut self, id: &str, title: &str) {
        let Some((root, repo_root)) = self.handle() else {
            return;
        };
        let presence = Presence {
            id: id.to_string(),
            title: title.to_string(),
            workspace: repo_root,
            last_activity: now_secs(),
        };
        if let Err(reason) = write_presence(&root, &presence) {
            self.degrade(reason, &root);
        }
    }

    /// Remove the presence record for the work item when it leaves active
    /// status, whichever workspace owns it: the item is no longer active
    /// anywhere, so a record owned by another workspace must not stay live
    /// until expiry — [[RFC-0010:C-PRESENCE]].
    pub fn remove(&mut self, id: &str) {
        let Some((root, _)) = self.handle() else {
            return;
        };
        let path = presence_path(&root, id);
        match read_presence(&path) {
            Ok(Some(_)) => {
                if let Err(err) = std::fs::remove_file(&path) {
                    self.warnings.push(Diagnostic::new(
                        DiagnosticCode::W0115RegistryDegraded,
                        format!(
                            "Could not remove the presence record for {id} ({err}); the \
                             stale record expires after the configured inactivity period."
                        ),
                        path.display().to_string(),
                    ));
                }
            }
            Ok(None) => {}
            Err(reason) => self.degrade(reason, &path),
        }
    }

    /// Record a corruption warning for an undecodable presence record; the
    /// read-only overlay skips the record and continues with the remaining
    /// ones — [[RFC-0010:C-REGISTRY]].
    fn warn_corrupt(&mut self, reason: String, path: &Path) {
        self.warnings.push(Diagnostic::new(
            DiagnosticCode::W0115RegistryDegraded,
            format!(
                "Detected a corrupt presence record ({reason}); the record is ignored. \
                 Cross-workspace presence visibility is degraded until the registry is \
                 repopulated."
            ),
            path.display().to_string(),
        ));
    }

    /// Live presence records owned by other workspaces of this clone, for the
    /// `govctl status` overlay — [[RFC-0010:C-PRESENCE]]. Undecodable records
    /// are skipped with a corruption warning — [[RFC-0010:C-REGISTRY]].
    pub fn foreign(&mut self) -> Vec<Presence> {
        let Some((root, repo_root)) = self.handle() else {
            return vec![];
        };
        let dir = root.join(PRESENCE_DIR);
        let now = now_secs();
        let mut records = Vec::new();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return records;
        };
        for path in entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        {
            match read_presence(&path) {
                Ok(Some(presence))
                    if presence.workspace != repo_root
                        && presence_is_live(&presence, self.ttl, now) =>
                {
                    records.push(presence);
                }
                Ok(_) => {}
                Err(reason) => self.warn_corrupt(reason, &path),
            }
        }
        records.sort_by(|left, right| left.id.cmp(&right.id));
        records
    }

    /// Warnings collected by this session.
    pub fn into_warnings(self) -> Vec<Diagnostic> {
        self.warnings
    }

    fn handle(&self) -> Option<(PathBuf, PathBuf)> {
        self.inner
            .as_ref()
            .map(|opened| (opened.root.clone(), opened.repo_root.clone()))
    }
}

/// Registry root for one governed project: a short stable hash of the
/// repo-relative gov root keeps monorepo projects isolated while all
/// workspaces of one project share the namespace.
fn registry_root(storage: &Path, gov_rel: &Path) -> PathBuf {
    let digest = Sha256::digest(gov_rel.to_string_lossy().as_bytes());
    let short: String = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    storage.join("govctl").join("registry").join(short)
}

fn sequence_of(id: &str, prefix: &str) -> Option<u32> {
    id.strip_prefix(prefix)?.parse().ok()
}

fn reservation_path(root: &Path, id: &str) -> PathBuf {
    root.join(RESERVATIONS_DIR).join(format!("{id}.toml"))
}

fn write_reservation(root: &Path, reservation: &Reservation) -> Result<(), String> {
    let path = reservation_path(root, &reservation.id);
    let content = toml::to_string_pretty(reservation)
        .map_err(|err| format!("cannot serialize the reservation: {err}"))?;
    // Atomic create: an existing record for the same ID means a stale or
    // conflicting reservation; refuse rather than silently overwrite.
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|err| format!("cannot create the reservation at {}: {err}", path.display()))?;
    io::Write::write_all(&mut file, content.as_bytes())
        .map_err(|err| format!("cannot write the reservation at {}: {err}", path.display()))
}

fn read_reservation(path: &Path) -> Result<Reservation, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| format!("cannot read the reservation at {}: {err}", path.display()))?;
    toml::from_str(&content)
        .map_err(|err| format!("corrupt reservation at {}: {err}", path.display()))
}

/// A free quarantine target for `path` under the registry's `corrupt/`
/// directory, keeping the original file name where possible. The caller
/// holds the allocation lock, so the existence check cannot race another
/// govctl process.
fn quarantine_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let dir = root.join(CORRUPT_DIR);
    std::fs::create_dir_all(&dir).map_err(|err| {
        format!(
            "cannot create the quarantine directory at {}: {err}",
            dir.display()
        )
    })?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| format!("cannot quarantine {}: no file name", path.display()))?;
    for attempt in 0..100u32 {
        let candidate = if attempt == 0 {
            dir.join(&name)
        } else {
            dir.join(format!("{name}.{attempt}"))
        };
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "cannot find a free quarantine name for {}",
        path.display()
    ))
}

/// Configured claim inactivity period — [[RFC-0010:C-ARTIFACT-CLAIM]].
fn claim_ttl(config: &Config) -> Duration {
    Duration::from_secs(config.workspace.claim_ttl_days.saturating_mul(24 * 60 * 60))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// A claim is live while its owning workspace still exists and its last
/// activity is within the inactivity period — [[RFC-0010:C-ARTIFACT-CLAIM]].
fn claim_is_live(claim: &Claim, ttl: Duration, now: u64) -> bool {
    claim.workspace.exists() && now.saturating_sub(claim.last_activity) <= ttl.as_secs()
}

/// A presence record follows the claim liveness rules —
/// [[RFC-0010:C-PRESENCE]].
fn presence_is_live(presence: &Presence, ttl: Duration, now: u64) -> bool {
    presence.workspace.exists() && now.saturating_sub(presence.last_activity) <= ttl.as_secs()
}

fn presence_path(root: &Path, id: &str) -> PathBuf {
    root.join(PRESENCE_DIR).join(format!("{id}.toml"))
}

/// Read a presence record; `Ok(None)` when no record exists for the work item.
fn read_presence(path: &Path) -> Result<Option<Presence>, String> {
    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content)
            .map(Some)
            .map_err(|err| format!("corrupt presence record at {}: {err}", path.display())),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!(
            "cannot read the presence record at {}: {err}",
            path.display()
        )),
    }
}

/// Write (or overwrite) a presence record. Overwrite is deliberate: presence
/// is advisory, so concurrent activations resolve to the latest registration
/// rather than conflicting.
fn write_presence(root: &Path, presence: &Presence) -> Result<(), String> {
    let dir = root.join(PRESENCE_DIR);
    std::fs::create_dir_all(&dir).map_err(|err| {
        format!(
            "cannot create the presence registry at {}: {err}",
            dir.display()
        )
    })?;
    let path = presence_path(root, &presence.id);
    let content = toml::to_string_pretty(presence)
        .map_err(|err| format!("cannot serialize the presence record: {err}"))?;
    std::fs::write(&path, content).map_err(|err| {
        format!(
            "cannot write the presence record at {}: {err}",
            path.display()
        )
    })
}

fn claim_path(root: &Path, id: &str) -> PathBuf {
    root.join(CLAIMS_DIR).join(format!("{id}.toml"))
}

/// Read a claim record; `Ok(None)` when no claim exists for the RFC.
fn read_claim(path: &Path) -> Result<Option<Claim>, String> {
    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content)
            .map(Some)
            .map_err(|err| format!("corrupt claim at {}: {err}", path.display())),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!(
            "cannot read the claim at {}: {err}",
            path.display()
        )),
    }
}

/// Write (or overwrite) a claim record. Overwrite is deliberate: claim
/// acquisition refreshes, re-acquires after expiry, and takes over.
fn write_claim(root: &Path, claim: &Claim) -> Result<(), String> {
    let dir = root.join(CLAIMS_DIR);
    std::fs::create_dir_all(&dir).map_err(|err| {
        format!(
            "cannot create the claims registry at {}: {err}",
            dir.display()
        )
    })?;
    let path = claim_path(root, &claim.id);
    let content = toml::to_string_pretty(claim)
        .map_err(|err| format!("cannot serialize the claim: {err}"))?;
    std::fs::write(&path, content)
        .map_err(|err| format!("cannot write the claim at {}: {err}", path.display()))
}

/// Append a takeover audit event; audit events are never pruned —
/// [[RFC-0010:C-ARTIFACT-CLAIM]].
fn write_audit_event(root: &Path, event: &AuditEvent) -> Result<(), String> {
    let dir = root.join(AUDIT_DIR);
    std::fs::create_dir_all(&dir).map_err(|err| {
        format!(
            "cannot create the audit registry at {}: {err}",
            dir.display()
        )
    })?;
    let content = toml::to_string_pretty(event)
        .map_err(|err| format!("cannot serialize the audit event: {err}"))?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    // create_new keeps events append-only; a nanosecond collision retries
    // with a suffix rather than overwriting a recorded takeover.
    for attempt in 0..100u32 {
        let name = if attempt == 0 {
            format!("{nanos}-{}.toml", event.id)
        } else {
            format!("{nanos}-{}-{attempt}.toml", event.id)
        };
        let path = dir.join(name);
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(mut file) => {
                return io::Write::write_all(&mut file, content.as_bytes()).map_err(|err| {
                    format!("cannot write the audit event at {}: {err}", path.display())
                });
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(format!(
                    "cannot create the audit event at {}: {err}",
                    path.display()
                ));
            }
        }
    }
    Err("cannot allocate a unique audit event name".to_string())
}

/// Refresh the last-activity timestamps of claims held by `repo_root`,
/// best-effort — [[RFC-0010:C-ARTIFACT-CLAIM]].
fn refresh_workspace_claims(root: &Path, repo_root: &Path) {
    let dir = root.join(CLAIMS_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for path in entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
    {
        let Ok(Some(mut claim)) = read_claim(&path) else {
            continue;
        };
        if claim.workspace == repo_root {
            claim.last_activity = now_secs();
            let _ = std::fs::write(&path, toml::to_string_pretty(&claim).unwrap_or_default());
        }
    }
}

/// Refresh the last-activity timestamps of presence records held by
/// `repo_root`, best-effort — [[RFC-0010:C-PRESENCE]].
fn refresh_workspace_presence(root: &Path, repo_root: &Path) {
    let dir = root.join(PRESENCE_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for path in entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
    {
        let Ok(Some(mut presence)) = read_presence(&path) else {
            continue;
        };
        if presence.workspace == repo_root {
            presence.last_activity = now_secs();
            let _ = std::fs::write(&path, toml::to_string_pretty(&presence).unwrap_or_default());
        }
    }
}

/// Diagnostic for a version-semantics operation blocked by a live claim
/// held by another workspace — [[RFC-0010:C-ARTIFACT-CLAIM]].
fn claim_conflict(id: &str, claim: &Claim, ttl: Duration) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0824ArtifactClaimed,
        format!(
            "Cannot change version semantics of {id}: the artifact claim is held by \
             workspace {}. Release the claim in that workspace, take it over with \
             `govctl claim steal {id}`, or wait for it to expire after {} days of \
             inactivity.",
            claim.workspace.display(),
            ttl.as_secs() / (24 * 60 * 60)
        ),
        id.to_string(),
    )
}

/// Collect artifact IDs from `git log --all --diff-filter=A -p -- <gov_rel>`:
/// every gov-tree file ever added to any ref witnesses its `id` line.
fn git_history_ids(repo_root: &Path, gov_rel: &Path) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(["log", "--all", "--diff-filter=A", "--format=", "-p", "--"])
        .arg(gov_rel)
        .current_dir(repo_root)
        .output()
        .map_err(|err| format!("cannot run git log: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "git log failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(extract_ids(&String::from_utf8_lossy(&output.stdout)))
}

/// Extract artifact IDs from added `id = "..."` lines of a diff.
fn extract_ids(diff: &str) -> Vec<String> {
    diff.lines()
        .filter_map(|line| {
            let line = line.strip_prefix('+')?.trim();
            let rest = line.strip_prefix("id")?.trim_start();
            let rest = rest.strip_prefix('=')?.trim_start();
            let id = rest.strip_prefix('"')?.strip_suffix('"')?;
            is_artifact_id(id).then(|| id.to_string())
        })
        .collect()
}

/// Whether `id` is a sequentially numbered artifact ID (RFC/ADR/work item).
fn is_artifact_id(id: &str) -> bool {
    numbered_id(id, "RFC-") || numbered_id(id, "ADR-") || work_id(id)
}

fn numbered_id(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix)
        .is_some_and(|num| num.len() == 4 && num.bytes().all(|byte| byte.is_ascii_digit()))
}

/// WI-YYYY-MM-DD-NNN, WI-YYYY-MM-DD-HHHH, or WI-YYYY-MM-DD-HHHH-NNN.
fn work_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("WI-") else {
        return false;
    };
    let mut segments = rest.split('-');
    let (Some(year), Some(month), Some(day)) = (segments.next(), segments.next(), segments.next())
    else {
        return false;
    };
    let date_ok = year.len() == 4
        && year.bytes().all(|byte| byte.is_ascii_digit())
        && [month, day]
            .into_iter()
            .all(|part| part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit()));
    if !date_ok {
        return false;
    }
    let sequence = |part: &str| part.len() == 3 && part.bytes().all(|b| b.is_ascii_digit());
    let hash = |part: &str| {
        part.len() == 4
            && part
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    };
    let tail: Vec<&str> = segments.collect();
    match tail.as_slice() {
        [part] => sequence(part) || hash(part),
        [author, seq] => hash(author) && sequence(seq),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_root_is_stable_and_namespaced() {
        let storage = Path::new("/storage");
        assert_eq!(
            registry_root(storage, Path::new("gov")),
            registry_root(storage, Path::new("gov"))
        );
        assert_ne!(
            registry_root(storage, Path::new("gov")),
            registry_root(storage, Path::new("services/api/gov"))
        );
        let root = registry_root(storage, Path::new("gov"));
        assert_eq!(root.parent().unwrap(), storage.join("govctl/registry"));
    }

    #[test]
    fn extract_ids_reads_added_artifact_id_lines() {
        let diff = "\
@@ -0,0 +1,5 @@
+[govctl]
+id = \"RFC-0012\"
+title = \"X\"
-id = \"ADR-0001\"
+id = \"ADR-0007\"
+id = \"WI-2026-09-08-002\"
+id = \"WI-2026-09-08-ab12-003\"
+id = \"{{ID}}\"
+id = \"WI-YYYY-MM-DD-NNN\"
 plain id = \"ADR-0009\"";
        let ids = extract_ids(diff);
        assert_eq!(
            ids,
            vec![
                "RFC-0012",
                "ADR-0007",
                "WI-2026-09-08-002",
                "WI-2026-09-08-ab12-003"
            ]
        );
    }

    #[test]
    fn artifact_id_patterns_match_generated_ids() {
        for id in [
            "RFC-0001",
            "ADR-9999",
            "WI-2026-09-08-002",
            "WI-2026-09-08-ab12",
            "WI-2026-09-08-ab12-003",
        ] {
            assert!(is_artifact_id(id), "{id}");
        }
        for id in [
            "RFC-001",
            "ADR-12345",
            "WI-2026-9-8-002",
            "WI-2026-09-08",
            "WI-2026-09-08-AB12",
            "GUARD-X",
            "CONF-X",
            "{{ID}}",
            "WI-YYYY-MM-DD-NNN",
        ] {
            assert!(!is_artifact_id(id), "{id}");
        }
    }

    #[test]
    fn sequence_of_strips_prefix() {
        assert_eq!(sequence_of("ADR-0007", "ADR-"), Some(7));
        assert_eq!(sequence_of("WI-2026-09-08-002", "WI-2026-09-08-"), Some(2));
        assert_eq!(sequence_of("WI-2026-09-08-ab12", "WI-2026-09-08-"), None);
        assert_eq!(sequence_of("RFC-0001", "ADR-"), None);
    }

    #[test]
    fn reservation_round_trips_through_toml() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join(RESERVATIONS_DIR)).unwrap();
        let reservation = Reservation {
            id: "ADR-0003".to_string(),
            workspace: PathBuf::from("/workspaces/main"),
            artifact: PathBuf::from("/workspaces/main/gov/adr/ADR-0003-x.toml"),
        };
        write_reservation(root, &reservation).unwrap();
        let read = read_reservation(&reservation_path(root, "ADR-0003")).unwrap();
        assert_eq!(read, reservation);
        // create_new refuses to overwrite an existing reservation.
        assert!(write_reservation(root, &reservation).is_err());
    }

    #[test]
    fn corrupt_reservation_is_an_error() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("broken.toml");
        std::fs::write(&path, "not = [valid").unwrap();
        assert!(read_reservation(&path).unwrap_err().contains("corrupt"));
    }

    #[test]
    fn quarantine_path_keeps_the_file_name_and_never_overwrites() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let source = root.join(RESERVATIONS_DIR).join("ADR-0001.toml");
        let first = quarantine_path(root, &source).unwrap();
        assert_eq!(first, root.join(CORRUPT_DIR).join("ADR-0001.toml"));
        // An existing quarantined record is preserved; the next quarantine of
        // the same file name picks a suffixed target.
        std::fs::write(&first, "broken").unwrap();
        let second = quarantine_path(root, &source).unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "broken");
    }

    #[test]
    fn claim_round_trips_and_overwrites() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let claim = Claim {
            id: "RFC-0001".to_string(),
            workspace: PathBuf::from("/workspaces/main"),
            last_activity: 100,
        };
        write_claim(root, &claim).unwrap();
        let read = read_claim(&claim_path(root, "RFC-0001")).unwrap().unwrap();
        assert_eq!(read, claim);
        // Claims overwrite: refresh, re-acquisition, and takeover rewrite.
        let refreshed = Claim {
            last_activity: 200,
            ..claim
        };
        write_claim(root, &refreshed).unwrap();
        let read = read_claim(&claim_path(root, "RFC-0001")).unwrap().unwrap();
        assert_eq!(read, refreshed);
        assert!(read_claim(&claim_path(root, "RFC-0002")).unwrap().is_none());
    }

    #[test]
    fn claim_liveness_requires_existing_workspace_within_ttl() {
        let temp = tempfile::TempDir::new().unwrap();
        let ttl = Duration::from_secs(7 * 24 * 60 * 60);
        let now = 1_000_000_000u64;
        let live = Claim {
            id: "RFC-0001".to_string(),
            workspace: temp.path().to_path_buf(),
            last_activity: now - 60,
        };
        assert!(claim_is_live(&live, ttl, now));
        // Expired by inactivity.
        assert!(!claim_is_live(&live, ttl, now + ttl.as_secs() + 1));
        // Expired because the owning workspace no longer exists.
        let missing = Claim {
            workspace: temp.path().join("gone"),
            ..live.clone()
        };
        assert!(!claim_is_live(&missing, ttl, now));
    }

    #[test]
    fn refresh_workspace_claims_touches_only_own_claims() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let own = Claim {
            id: "RFC-0001".to_string(),
            workspace: temp.path().to_path_buf(),
            last_activity: 100,
        };
        let foreign = Claim {
            id: "RFC-0002".to_string(),
            workspace: PathBuf::from("/workspaces/other"),
            last_activity: 100,
        };
        write_claim(root, &own).unwrap();
        write_claim(root, &foreign).unwrap();
        refresh_workspace_claims(root, temp.path());
        let own = read_claim(&claim_path(root, "RFC-0001")).unwrap().unwrap();
        let foreign = read_claim(&claim_path(root, "RFC-0002")).unwrap().unwrap();
        assert!(own.last_activity > 100);
        assert_eq!(foreign.last_activity, 100);
    }

    #[test]
    fn presence_round_trips_and_overwrites() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let presence = Presence {
            id: "WI-2026-09-08-004".to_string(),
            title: "Presence overlay".to_string(),
            workspace: PathBuf::from("/workspaces/main"),
            last_activity: 100,
        };
        write_presence(root, &presence).unwrap();
        let read = read_presence(&presence_path(root, "WI-2026-09-08-004"))
            .unwrap()
            .unwrap();
        assert_eq!(read, presence);
        // Presence is advisory: re-registration overwrites, never conflicts.
        let refreshed = Presence {
            last_activity: 200,
            ..presence
        };
        write_presence(root, &refreshed).unwrap();
        let read = read_presence(&presence_path(root, "WI-2026-09-08-004"))
            .unwrap()
            .unwrap();
        assert_eq!(read, refreshed);
        assert!(
            read_presence(&presence_path(root, "WI-2026-09-08-005"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn presence_liveness_requires_existing_workspace_within_ttl() {
        let temp = tempfile::TempDir::new().unwrap();
        let ttl = Duration::from_secs(7 * 24 * 60 * 60);
        let now = 1_000_000_000u64;
        let live = Presence {
            id: "WI-2026-09-08-004".to_string(),
            title: "Presence overlay".to_string(),
            workspace: temp.path().to_path_buf(),
            last_activity: now - 60,
        };
        assert!(presence_is_live(&live, ttl, now));
        // Expired by inactivity.
        assert!(!presence_is_live(&live, ttl, now + ttl.as_secs() + 1));
        // Expired because the owning workspace no longer exists.
        let missing = Presence {
            workspace: temp.path().join("gone"),
            ..live.clone()
        };
        assert!(!presence_is_live(&missing, ttl, now));
    }

    #[test]
    fn refresh_workspace_presence_touches_only_own_records() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let own = Presence {
            id: "WI-2026-09-08-004".to_string(),
            title: "Own".to_string(),
            workspace: temp.path().to_path_buf(),
            last_activity: 100,
        };
        let foreign = Presence {
            id: "WI-2026-09-08-005".to_string(),
            title: "Foreign".to_string(),
            workspace: PathBuf::from("/workspaces/other"),
            last_activity: 100,
        };
        write_presence(root, &own).unwrap();
        write_presence(root, &foreign).unwrap();
        refresh_workspace_presence(root, temp.path());
        let own = read_presence(&presence_path(root, "WI-2026-09-08-004"))
            .unwrap()
            .unwrap();
        let foreign = read_presence(&presence_path(root, "WI-2026-09-08-005"))
            .unwrap()
            .unwrap();
        assert!(own.last_activity > 100);
        assert_eq!(foreign.last_activity, 100);
    }

    #[test]
    fn audit_events_accumulate_and_are_never_pruned() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        for id in ["RFC-0001", "RFC-0001"] {
            let event = AuditEvent {
                event: "takeover".to_string(),
                id: id.to_string(),
                from_workspace: PathBuf::from("/workspaces/a"),
                to_workspace: PathBuf::from("/workspaces/b"),
                timestamp: 100,
            };
            write_audit_event(root, &event).unwrap();
        }
        let events = std::fs::read_dir(root.join(AUDIT_DIR)).unwrap().count();
        assert_eq!(events, 2);
    }
}
