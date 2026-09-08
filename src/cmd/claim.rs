//! Artifact claim management commands — [[RFC-0010:C-ARTIFACT-CLAIM]].
//!
//! Claims are local coordination state in the clone's shared VCS storage;
//! these commands never mutate governed artifacts. `list` shows claims held
//! across the clone's workspaces, `release` frees a claim held by this
//! workspace, and `steal` takes over a claim held by another workspace,
//! recording an audit event that names both workspaces.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::registry::{ClaimSession, ReleaseOutcome, StealOutcome};
use crate::ui;
use crate::write::WriteOp;

/// List all claims (live and expired) held across this clone's workspaces.
/// Read-only and lock-free per [[RFC-0010:C-REGISTRY]].
pub fn list(config: &Config) -> DiagnosticResult<Diagnostics> {
    let mut session = ClaimSession::begin_readonly(config);
    if session.is_degraded() {
        return Err(registry_unavailable());
    }
    if !session.is_active() {
        ui::info("No artifact claims (no version control; coordination is inactive)");
        return Ok(vec![]);
    }
    let claims = session.list();
    if claims.is_empty() {
        ui::info("No artifact claims");
        return Ok(session.into_warnings());
    }
    let now = now_secs();
    for claim in claims {
        let state = if claim.live { "live" } else { "expired" };
        println!(
            "{}  {}  last activity {}  ({state})",
            claim.id,
            claim.workspace.display(),
            format_age(now.saturating_sub(claim.last_activity)),
        );
    }
    Ok(session.into_warnings())
}

/// Explicitly release a claim held by this workspace.
pub fn release(config: &Config, id: &str, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    require_rfc_id(id)?;
    let mut session = ClaimSession::begin(config, op.is_preview());
    if session.is_degraded() {
        return Err(registry_unavailable());
    }
    if op.is_preview() {
        ui::info(format!("Dry run: would release the claim on {id}"));
        return Ok(vec![]);
    }
    match session.release(id)? {
        ReleaseOutcome::Released => ui::success(format!("Released claim on {id}")),
        ReleaseOutcome::NotHeld => ui::info(format!("No claim on {id}")),
    }
    Ok(session.into_warnings())
}

/// Take over a claim held by another workspace, recording an audit event.
pub fn steal(config: &Config, id: &str, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    require_rfc_id(id)?;
    let mut session = ClaimSession::begin(config, op.is_preview());
    if session.is_degraded() {
        return Err(registry_unavailable());
    }
    if op.is_preview() {
        ui::info(format!("Dry run: would take over the claim on {id}"));
        return Ok(vec![]);
    }
    if !session.is_active() {
        ui::info("No version control; artifact claims are inactive and need no takeover");
        return Ok(vec![]);
    }
    match session.steal(id)? {
        StealOutcome::Taken => ui::success(format!(
            "Claim on {id} transferred to this workspace (takeover recorded)"
        )),
        StealOutcome::AlreadyHeld => {
            ui::info(format!("This workspace already holds the claim on {id}"))
        }
    }
    Ok(session.into_warnings())
}

/// Claims bind RFCs only — [[RFC-0010:C-ARTIFACT-CLAIM]].
fn require_rfc_id(id: &str) -> DiagnosticResult<()> {
    if crate::load::valid_rfc_id(id) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E0110RfcInvalidId,
            format!("Invalid RFC ID: {id} (expected RFC-NNNN)"),
            id,
        ))
    }
}

fn registry_unavailable() -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0822UnsupportedOperation,
        "Cannot manage artifact claims: the coordination registry is unavailable",
        "claim command",
    )
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn format_age(age_secs: u64) -> String {
    let days = age_secs / (24 * 60 * 60);
    if days > 0 {
        format!("{days}d ago")
    } else {
        let hours = age_secs / (60 * 60);
        if hours > 0 {
            format!("{hours}h ago")
        } else {
            format!("{}m ago", age_secs / 60)
        }
    }
}
