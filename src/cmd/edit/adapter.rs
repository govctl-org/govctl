//! Format adapters for artifact edit operations (ADR-0031).
//!
//! These adapters provide a stable read/write boundary for JSON and TOML
//! artifacts while execution is migrated from legacy dispatch to V2 engine.

use crate::config::Config;
use crate::load::{find_clause_json, find_rfc_json};
use crate::model::{AdrEntry, ClauseSpec, RfcSpec, WorkItemEntry};
use crate::parse::{load_adrs, load_work_items, write_adr, write_work_item};
use crate::write::{WriteOp, read_clause, read_rfc, write_clause, write_rfc};
use std::path::PathBuf;

/// Generic JSON document container (path + parsed payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDoc<T> {
    pub path: PathBuf,
    pub data: T,
}

/// Adapter contract for JSON-backed artifacts.
#[allow(dead_code)]
pub trait JsonAdapter {
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

impl JsonAdapter for RfcJsonAdapter {
    type Data = RfcSpec;

    fn load(config: &Config, id: &str) -> anyhow::Result<JsonDoc<Self::Data>> {
        let path =
            find_rfc_json(config, id).ok_or_else(|| anyhow::anyhow!("RFC not found: {id}"))?;
        let data = read_rfc(config, &path)?;
        Ok(JsonDoc { path, data })
    }

    fn write(config: &Config, doc: &JsonDoc<Self::Data>, op: WriteOp) -> anyhow::Result<()> {
        write_rfc(
            &doc.path,
            &doc.data,
            op,
            Some(&config.display_path(&doc.path)),
        )
    }
}

/// Clause JSON adapter.
pub struct ClauseJsonAdapter;

impl JsonAdapter for ClauseJsonAdapter {
    type Data = ClauseSpec;

    fn load(config: &Config, id: &str) -> anyhow::Result<JsonDoc<Self::Data>> {
        let path = find_clause_json(config, id)
            .ok_or_else(|| anyhow::anyhow!("Clause not found: {id}"))?;
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
        load_adrs(config)?
            .into_iter()
            .find(|a| a.spec.govctl.id == id)
            .ok_or_else(|| anyhow::anyhow!("ADR not found: {id}"))
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
        load_work_items(config)?
            .into_iter()
            .find(|w| w.spec.govctl.id == id || w.path.to_string_lossy().contains(id))
            .ok_or_else(|| anyhow::anyhow!("Work item not found: {id}"))
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
