use super::adapter::{DocAdapter, JsonDoc, display_scope_for_dir};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::load::{find_clause_json, find_clause_toml, find_rfc_json, find_rfc_toml};
use crate::model::{ClauseSpec, RfcSpec};
use crate::write::{WriteOp, read_clause, read_rfc, write_clause, write_rfc};
use std::path::PathBuf;

fn load_rfc_with<F>(config: &Config, id: &str, finder: F) -> DiagnosticResult<JsonDoc<RfcSpec>>
where
    F: Fn(&Config, &str) -> Option<PathBuf>,
{
    let scope = display_scope_for_dir(config, config.rfc_dir());
    let path = finder(config, id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {id}"),
            &scope,
        )
    })?;
    let data = read_rfc(config, &path)?;
    Ok(JsonDoc { path, data })
}

fn write_rfc_doc(config: &Config, doc: &JsonDoc<RfcSpec>, op: WriteOp) -> DiagnosticResult<()> {
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
        .map(|rfc_id| config.rfc_dir().join(rfc_id).join("clauses"))
        .unwrap_or_else(|| config.rfc_dir())
}

fn load_clause_with<F>(
    config: &Config,
    id: &str,
    finder: F,
) -> DiagnosticResult<JsonDoc<ClauseSpec>>
where
    F: Fn(&Config, &str) -> Option<PathBuf>,
{
    let scope = display_scope_for_dir(config, clause_scope_path(config, id));
    let path = finder(config, id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Clause not found: {id}"),
            &scope,
        )
    })?;
    let data = read_clause(config, &path)?;
    Ok(JsonDoc { path, data })
}

fn write_clause_doc(
    config: &Config,
    doc: &JsonDoc<ClauseSpec>,
    op: WriteOp,
) -> DiagnosticResult<()> {
    write_clause(
        &doc.path,
        &doc.data,
        op,
        Some(&config.display_path(&doc.path)),
    )
}

/// RFC JSON adapter.
pub struct RfcJsonAdapter;

impl DocAdapter for RfcJsonAdapter {
    type Data = RfcSpec;

    fn load(config: &Config, id: &str) -> DiagnosticResult<JsonDoc<Self::Data>> {
        load_rfc_with(config, id, find_rfc_json)
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> DiagnosticResult<()> {
        write_rfc_doc(config, doc, op)
    }
}

pub struct RfcTomlAdapter;

impl DocAdapter for RfcTomlAdapter {
    type Data = RfcSpec;

    fn load(config: &Config, id: &str) -> DiagnosticResult<JsonDoc<Self::Data>> {
        load_rfc_with(config, id, find_rfc_toml)
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> DiagnosticResult<()> {
        write_rfc_doc(config, doc, op)
    }
}

/// Clause JSON adapter.
pub struct ClauseJsonAdapter;

impl DocAdapter for ClauseJsonAdapter {
    type Data = ClauseSpec;

    fn load(config: &Config, id: &str) -> DiagnosticResult<JsonDoc<Self::Data>> {
        load_clause_with(config, id, find_clause_json)
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> DiagnosticResult<()> {
        write_clause_doc(config, doc, op)
    }
}

pub struct ClauseTomlAdapter;

impl DocAdapter for ClauseTomlAdapter {
    type Data = ClauseSpec;

    fn load(config: &Config, id: &str) -> DiagnosticResult<JsonDoc<Self::Data>> {
        load_clause_with(config, id, find_clause_toml)
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> DiagnosticResult<()> {
        write_clause_doc(config, doc, op)
    }
}
