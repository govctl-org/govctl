use crate::config::Config;
use crate::diagnostic::DiagnosticResult;
use crate::write::WriteOp;
use std::path::PathBuf;

pub use super::doc_adapter::{ClauseTomlAdapter, RfcTomlAdapter};
pub use super::toml_adapter::{AdrTomlAdapter, GuardTomlAdapter, WorkTomlAdapter};

/// Generic document container (path + parsed payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDoc<T> {
    pub path: PathBuf,
    pub data: T,
}

pub(super) fn display_scope_for_dir(config: &Config, path: PathBuf) -> String {
    config.display_path(&path).display().to_string()
}

/// Adapter contract for RFC/clause document-backed artifacts.
pub trait DocAdapter {
    type Data;

    fn load(config: &Config, id: &str) -> DiagnosticResult<LoadedDoc<Self::Data>>;
    fn write(config: &Config, doc: &LoadedDoc<Self::Data>, op: WriteOp) -> DiagnosticResult<()>;
}

/// Adapter contract for TOML-backed artifacts.
pub trait TomlAdapter {
    type Entry;

    fn load(config: &Config, id: &str) -> DiagnosticResult<Self::Entry>;
    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> DiagnosticResult<()>;
}
