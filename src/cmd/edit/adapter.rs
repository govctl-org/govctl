//! Format adapters for artifact edit operations (ADR-0031).
//!
//! These adapters provide a stable read/write boundary for JSON and TOML
//! artifacts while execution is migrated from legacy dispatch to V2 engine.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::{find_clause_json, find_clause_toml, find_rfc_json, find_rfc_toml};
use crate::model::{AdrEntry, ClauseSpec, GuardEntry, RfcSpec, WorkItemEntry};
use crate::parse::{
    load_adrs, load_guards, load_work_items, write_adr, write_guard, write_work_item,
};
use crate::write::{WriteOp, read_clause, read_rfc, write_clause, write_rfc};
use std::path::PathBuf;

/// Generic JSON document container (path + parsed payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDoc<T> {
    pub path: PathBuf,
    pub data: T,
}

fn display_scope_for_dir(config: &Config, path: PathBuf) -> String {
    config.display_path(&path).display().to_string()
}

fn load_rfc_with<F>(config: &Config, id: &str, finder: F) -> anyhow::Result<JsonDoc<RfcSpec>>
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

fn write_rfc_doc(config: &Config, doc: &JsonDoc<RfcSpec>, op: WriteOp) -> anyhow::Result<()> {
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

/// Adapter contract for RFC/clause document-backed artifacts.
#[allow(dead_code)]
pub trait DocAdapter {
    type Data;

    fn load(config: &Config, id: &str) -> anyhow::Result<JsonDoc<Self::Data>>;
    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> anyhow::Result<()>;
}

/// Adapter contract for TOML-backed artifacts.
#[allow(dead_code)]
pub trait TomlAdapter {
    type Entry;

    fn load(config: &Config, id: &str) -> anyhow::Result<Self::Entry>;
    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> anyhow::Result<()>;
}

/// RFC JSON adapter.
pub struct RfcJsonAdapter;

impl DocAdapter for RfcJsonAdapter {
    type Data = RfcSpec;

    fn load(config: &Config, id: &str) -> anyhow::Result<JsonDoc<Self::Data>> {
        load_rfc_with(config, id, find_rfc_json)
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> anyhow::Result<()> {
        write_rfc_doc(config, doc, op)
    }
}

pub struct RfcTomlAdapter;

impl DocAdapter for RfcTomlAdapter {
    type Data = RfcSpec;

    fn load(config: &Config, id: &str) -> anyhow::Result<JsonDoc<Self::Data>> {
        load_rfc_with(config, id, find_rfc_toml)
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> anyhow::Result<()> {
        write_rfc_doc(config, doc, op)
    }
}

/// Clause JSON adapter.
pub struct ClauseJsonAdapter;

impl DocAdapter for ClauseJsonAdapter {
    type Data = ClauseSpec;

    fn load(config: &Config, id: &str) -> anyhow::Result<JsonDoc<Self::Data>> {
        let scope = display_scope_for_dir(config, clause_scope_path(config, id));
        let path = find_clause_json(config, id).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {id}"),
                &scope,
            )
        })?;
        let data = read_clause(config, &path)?;
        Ok(JsonDoc { path, data })
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> anyhow::Result<()> {
        write_clause(
            &doc.path,
            &doc.data,
            op,
            Some(&config.display_path(&doc.path)),
        )
    }
}

pub struct ClauseTomlAdapter;

impl DocAdapter for ClauseTomlAdapter {
    type Data = ClauseSpec;

    fn load(config: &Config, id: &str) -> anyhow::Result<JsonDoc<Self::Data>> {
        let scope = display_scope_for_dir(config, clause_scope_path(config, id));
        let path = find_clause_toml(config, id).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {id}"),
                &scope,
            )
        })?;
        let data = read_clause(config, &path)?;
        Ok(JsonDoc { path, data })
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> anyhow::Result<()> {
        write_clause(
            &doc.path,
            &doc.data,
            op,
            Some(&config.display_path(&doc.path)),
        )
    }
}

/// ADR TOML adapter.
pub struct AdrTomlAdapter;

impl TomlAdapter for AdrTomlAdapter {
    type Entry = AdrEntry;

    fn load(config: &Config, id: &str) -> anyhow::Result<Self::Entry> {
        let scope = config.display_path(&config.adr_dir()).display().to_string();
        Ok(load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0302AdrNotFound,
                    format!("ADR not found: {id}"),
                    &scope,
                )
            })?)
    }

    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> anyhow::Result<()> {
        write_adr(
            &entry.path,
            &entry.spec,
            op,
            Some(&config.display_path(&entry.path)),
        )
        .map_err(Into::into)
    }
}

/// Work item TOML adapter.
pub struct WorkTomlAdapter;

impl TomlAdapter for WorkTomlAdapter {
    type Entry = WorkItemEntry;

    fn load(config: &Config, id: &str) -> anyhow::Result<Self::Entry> {
        let scope = config
            .display_path(&config.work_dir())
            .display()
            .to_string();
        Ok(load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0402WorkNotFound,
                    format!("Work item not found: {id}"),
                    &scope,
                )
            })?)
    }

    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> anyhow::Result<()> {
        write_work_item(
            &entry.path,
            &entry.spec,
            op,
            Some(&config.display_path(&entry.path)),
        )
        .map_err(Into::into)
    }
}

/// Guard TOML adapter.
pub struct GuardTomlAdapter;

impl TomlAdapter for GuardTomlAdapter {
    type Entry = GuardEntry;

    fn load(config: &Config, id: &str) -> anyhow::Result<Self::Entry> {
        let scope = config
            .display_path(&config.guard_dir())
            .display()
            .to_string();
        Ok(load_guards(config)
            .map_err(anyhow::Error::from)?
            .into_iter()
            .find(|g| g.spec.govctl.id == id)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E1002GuardNotFound,
                    format!("Guard not found: {id}"),
                    &scope,
                )
            })?)
    }

    fn write(config: &Config, entry: &Self::Entry, op: WriteOp) -> anyhow::Result<()> {
        write_guard(
            &entry.path,
            &entry.spec,
            op,
            Some(&config.display_path(&entry.path)),
        )
        .map_err(Into::into)
    }
}
