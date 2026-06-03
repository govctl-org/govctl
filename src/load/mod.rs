//! Governance artifact loading and project-index assembly.

use crate::diagnostic::{Diagnostic, DiagnosticCode};

mod project;
mod rfc;

pub use project::{load_project, load_project_with_warnings};
pub(crate) use rfc::split_clause_id;
pub use rfc::{find_clause_toml, find_rfc_toml, load_rfc, load_rfcs, reject_legacy_json_storage};

/// Result of loading a project: index plus any warnings encountered
pub struct ProjectLoadResult {
    pub index: crate::model::ProjectIndex,
    pub warnings: Vec<Diagnostic>,
}

/// Load error types
#[derive(Debug)]
pub enum LoadError {
    Io {
        file: String,
        action: &'static str,
        message: String,
    },
    InternalIo {
        file: String,
        message: String,
    },
    Json {
        file: String,
        message: String,
    },
    RfcSchema {
        file: String,
        message: String,
    },
    ClauseSchema {
        file: String,
        message: String,
    },
    ClausePathInvalid {
        file: String,
        clause: String,
    },
    Diagnostic(Diagnostic),
}

impl From<LoadError> for Diagnostic {
    fn from(err: LoadError) -> Self {
        match err {
            LoadError::Io {
                file,
                action,
                message,
            } => Diagnostic::io_error(action, message, file),
            LoadError::InternalIo { file, message } => {
                Diagnostic::new(DiagnosticCode::E0901IoError, message, file)
            }
            LoadError::Json { file, message } => {
                Diagnostic::new(DiagnosticCode::E0902JsonParseError, message, file)
            }
            LoadError::RfcSchema { file, message } => {
                Diagnostic::new(DiagnosticCode::E0101RfcSchemaInvalid, message, file)
            }
            LoadError::ClauseSchema { file, message } => {
                Diagnostic::new(DiagnosticCode::E0201ClauseSchemaInvalid, message, file)
            }
            LoadError::ClausePathInvalid { file, clause } => Diagnostic::new(
                DiagnosticCode::E0204ClausePathInvalid,
                format!("Invalid clause path: {clause}"),
                file,
            ),
            LoadError::Diagnostic(diagnostic) => diagnostic,
        }
    }
}

pub fn load_clause(
    config: &crate::config::Config,
    path: &std::path::Path,
) -> Result<crate::model::ClauseEntry, LoadError> {
    rfc::load_clause_file(config, path)
}
