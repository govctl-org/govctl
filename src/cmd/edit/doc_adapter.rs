use super::adapter::{DocAdapter, LoadedDoc, display_scope_for_dir};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::load::{find_clause_toml, find_rfc_toml};
use crate::model::{ClauseSpec, RfcSpec};
use crate::write::{WriteOp, read_clause, read_rfc, write_clause, write_rfc};
use std::path::{Path, PathBuf};

fn load_doc_with<T, F, R>(
    config: &Config,
    id: &str,
    finder: F,
    scope_path: PathBuf,
    missing_code: DiagnosticCode,
    missing_label: &str,
    read: R,
) -> DiagnosticResult<LoadedDoc<T>>
where
    F: Fn(&Config, &str) -> Option<PathBuf>,
    R: Fn(&Config, &Path) -> DiagnosticResult<T>,
{
    let scope = display_scope_for_dir(config, scope_path);
    let path = finder(config, id).ok_or_else(|| {
        Diagnostic::new(
            missing_code,
            format!("{missing_label} not found: {id}"),
            &scope,
        )
    })?;
    let data = read(config, &path)?;
    Ok(LoadedDoc { path, data })
}

fn load_rfc_with<F>(config: &Config, id: &str, finder: F) -> DiagnosticResult<LoadedDoc<RfcSpec>>
where
    F: Fn(&Config, &str) -> Option<PathBuf>,
{
    load_doc_with(
        config,
        id,
        finder,
        config.rfc_dir(),
        DiagnosticCode::E0102RfcNotFound,
        "RFC",
        read_rfc,
    )
}

fn write_rfc_doc(config: &Config, doc: &LoadedDoc<RfcSpec>, op: WriteOp) -> DiagnosticResult<()> {
    write_rfc(
        &doc.path,
        &doc.data,
        op,
        Some(&config.display_path(&doc.path)),
    )
}

fn clause_scope_path(config: &Config, id: &str) -> PathBuf {
    id.split(':')
        .next()
        .map(|rfc_id| config.clause_dir(rfc_id))
        .unwrap_or_else(|| config.rfc_dir())
}

fn load_clause_with<F>(
    config: &Config,
    id: &str,
    finder: F,
) -> DiagnosticResult<LoadedDoc<ClauseSpec>>
where
    F: Fn(&Config, &str) -> Option<PathBuf>,
{
    load_doc_with(
        config,
        id,
        finder,
        clause_scope_path(config, id),
        DiagnosticCode::E0202ClauseNotFound,
        "Clause",
        read_clause,
    )
}

fn write_clause_doc(
    config: &Config,
    doc: &LoadedDoc<ClauseSpec>,
    op: WriteOp,
) -> DiagnosticResult<()> {
    write_clause(
        &doc.path,
        &doc.data,
        op,
        Some(&config.display_path(&doc.path)),
    )
}

pub struct RfcTomlAdapter;

impl DocAdapter for RfcTomlAdapter {
    type Data = RfcSpec;

    fn load(config: &Config, id: &str) -> DiagnosticResult<LoadedDoc<Self::Data>> {
        load_rfc_with(config, id, find_rfc_toml)
    }

    fn write(config: &Config, doc: &LoadedDoc<Self::Data>, op: WriteOp) -> DiagnosticResult<()> {
        write_rfc_doc(config, doc, op)
    }
}

pub struct ClauseTomlAdapter;

impl DocAdapter for ClauseTomlAdapter {
    type Data = ClauseSpec;

    fn load(config: &Config, id: &str) -> DiagnosticResult<LoadedDoc<Self::Data>> {
        load_clause_with(config, id, find_clause_toml)
    }

    fn write(config: &Config, doc: &LoadedDoc<Self::Data>, op: WriteOp) -> DiagnosticResult<()> {
        write_clause_doc(config, doc, op)
    }
}
