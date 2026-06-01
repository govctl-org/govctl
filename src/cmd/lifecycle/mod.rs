//! Lifecycle command implementations.

use crate::FinalizeStatus;
use crate::cmd::confirmation::confirm_destructive_action;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::write::WriteOp;

mod adr;
mod clause;
mod release;
mod rfc;
pub use adr::{accept_adr, reject_adr, validate_adr_completeness};
pub use release::cut_release;
pub use rfc::{advance, bump, finalize};

/// Deprecate an artifact
///
/// Per [[ADR-0017]], destructive operations require confirmation unless `--force`.
pub fn deprecate(
    config: &Config,
    id: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if !confirm_destructive_action(
        force,
        op,
        &format!("Deprecate {}?", id),
        "Deprecation cancelled",
    )? {
        return Ok(vec![]);
    }

    if id.contains(':') {
        clause::deprecate_clause(config, id, op)
    } else if id.starts_with("RFC-") {
        finalize(config, id, FinalizeStatus::Deprecated, op)
    } else if id.starts_with("ADR-") {
        Err(Diagnostic::new(
            DiagnosticCode::E0305AdrCannotDeprecate,
            format!(
                "ADRs cannot be deprecated. Use `govctl supersede {id} --by ADR-XXXX` instead."
            ),
            id,
        ))
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Unknown artifact type: {id}"),
            id,
        ))
    }
}

/// Supersede an artifact
///
/// Per [[ADR-0017]], destructive operations require confirmation unless `--force`.
pub fn supersede(
    config: &Config,
    id: &str,
    by: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if !confirm_destructive_action(
        force,
        op,
        &format!("Supersede {} with {}?", id, by),
        "Supersede cancelled",
    )? {
        return Ok(vec![]);
    }

    if id.contains(':') {
        clause::supersede_clause(config, id, by, op)
    } else if id.starts_with("RFC-") {
        rfc::supersede_rfc(config, id, by, op)
    } else if id.starts_with("ADR-") {
        adr::supersede_adr(config, id, by, op)
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Supersede is not supported for this artifact type: {id}"),
            id,
        ))
    }
}
