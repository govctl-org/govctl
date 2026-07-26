//! RFC and clause artifact read/write helpers.

use super::WriteOp;
use super::artifact_io::{ArtifactIo, read_artifact, write_toml_artifact};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{ClauseSpec, ClauseWire, RfcSpec, RfcWire};
use crate::schema::ArtifactSchema;
use std::path::Path;

const RFC_IO: ArtifactIo = ArtifactIo {
    read_label: "RFC",
    message_label: "RFC",
    schema: ArtifactSchema::Rfc,
    schema_error: DiagnosticCode::E0101RfcSchemaInvalid,
};

const CLAUSE_IO: ArtifactIo = ArtifactIo {
    read_label: "clause",
    message_label: "clause",
    schema: ArtifactSchema::Clause,
    schema_error: DiagnosticCode::E0201ClauseSchemaInvalid,
};

/// Read an RFC from canonical structured TOML.
pub fn read_rfc(config: &Config, path: &Path) -> DiagnosticResult<RfcSpec> {
    crate::load::validate_rfc_storage_path(config, path).map_err(Diagnostic::from)?;
    read_artifact::<RfcWire, RfcSpec>(config, path, &RFC_IO)
}

/// Write RFC to file in TOML only.
/// TOML output uses the `[govctl]` wire format plus schema header.
pub fn write_rfc(
    config: &Config,
    path: &Path,
    rfc: &RfcSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    crate::load::validate_rfc_storage_path(config, path).map_err(Diagnostic::from)?;
    let wire: RfcWire = rfc.clone().into();
    write_toml_artifact(
        path,
        &wire,
        ArtifactSchema::Rfc,
        DiagnosticCode::E0101RfcSchemaInvalid,
        "RFC",
        op,
        display_path,
    )
}

/// Read a clause from canonical structured TOML.
pub fn read_clause(config: &Config, path: &Path) -> DiagnosticResult<ClauseSpec> {
    crate::load::validate_rfc_storage_path(config, path).map_err(Diagnostic::from)?;
    read_artifact::<ClauseWire, ClauseSpec>(config, path, &CLAUSE_IO)
}

/// Write clause to file in TOML only.
/// TOML output uses the `[govctl]` + `[content]` wire format plus schema header.
pub fn write_clause(
    config: &Config,
    path: &Path,
    clause: &ClauseSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> DiagnosticResult<()> {
    crate::load::validate_rfc_storage_path(config, path).map_err(Diagnostic::from)?;
    let wire: ClauseWire = clause.clone().into();
    write_toml_artifact(
        path,
        &wire,
        ArtifactSchema::Clause,
        DiagnosticCode::E0201ClauseSchemaInvalid,
        "clause",
        op,
        display_path,
    )
}
