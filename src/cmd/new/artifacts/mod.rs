//! Artifact creation helpers for the `new` command.

mod adr;
mod clause;
mod rfc;
mod work;

use crate::NewTarget;
use crate::config::Config;
use crate::diagnostic::{DiagnosticResult, Diagnostics};
use crate::write::WriteOp;

/// Create a new artifact.
pub fn create(config: &Config, target: &NewTarget, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    match target {
        NewTarget::Rfc { title, id } => rfc::create(config, title, id.as_deref(), op),
        NewTarget::Clause {
            clause_id,
            title,
            section,
            kind,
        } => clause::create(config, clause_id, title, section, *kind, op),
        NewTarget::Adr { title } => adr::create(config, title, op),
        NewTarget::Work { title, active } => work::create(config, title, *active, op),
    }
}
