//! Artifact creation helpers for the `new` command.

mod adr;
mod clause;
mod rfc;
mod work;

use crate::NewTarget;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::schema::{ArtifactSchema, with_schema_header};
use crate::write::{WriteOp, write_file};
use serde::Serialize;
use std::path::Path;

pub(super) fn write_new_artifact_toml<T: Serialize>(
    config: &Config,
    path: &Path,
    value: &T,
    schema: ArtifactSchema,
    schema_error: DiagnosticCode,
    label: &str,
    op: WriteOp,
) -> DiagnosticResult<()> {
    let display_path = config.display_path(path);
    let body = toml::to_string_pretty(value).map_err(|err| {
        Diagnostic::new(
            schema_error,
            format!("Failed to serialize {label} TOML: {err}"),
            display_path.display().to_string(),
        )
    })?;
    let content = with_schema_header(schema, &body);
    write_file(path, &content, op, Some(&display_path))
}

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
