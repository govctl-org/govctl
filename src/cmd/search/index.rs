use super::document::{self, SearchDocument};
use crate::artifact_catalog::{self, CatalogKind, CatalogRecord};
use crate::config::Config;
use crate::diagnostic::DiagnosticResult;
use crate::local_index::{database_path, open_database, sqlite_diagnostic};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};
use std::path::Path;

const SEARCH_SCHEMA_VERSION: i64 = 2;

pub(super) fn open(config: &Config) -> DiagnosticResult<Connection> {
    let (connection, path) = open_database(config, "open search index")?;
    initialize_schema(&connection, &path)?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection, path: &Path) -> DiagnosticResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS search_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|error| sqlite_diagnostic("initialize search metadata", error, path))?;

    let version: Option<i64> = connection
        .query_row(
            "SELECT value FROM search_meta WHERE key = 'search_schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| sqlite_diagnostic("read search schema version", error, path))?
        .and_then(|value| value.parse::<i64>().ok());

    if version != Some(SEARCH_SCHEMA_VERSION) {
        connection
            .execute_batch(
                "
                DROP TABLE IF EXISTS search_fts;
                DROP TABLE IF EXISTS search_manifest;
                DELETE FROM search_meta WHERE key = 'search_schema_version';
                ",
            )
            .map_err(|error| sqlite_diagnostic("reset search index", error, path))?;
    }

    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS search_manifest (
                kind TEXT NOT NULL,
                id TEXT NOT NULL,
                path TEXT NOT NULL,
                mtime_ns INTEGER NOT NULL,
                size INTEGER NOT NULL,
                source_hash TEXT NOT NULL,
                PRIMARY KEY (kind, id)
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
                kind UNINDEXED,
                id UNINDEXED,
                title,
                path UNINDEXED,
                tags UNINDEXED,
                status UNINDEXED,
                body,
                tokenize = 'porter unicode61'
            );
            ",
        )
        .map_err(|error| sqlite_diagnostic("initialize search index", error, path))?;

    connection
        .execute(
            "INSERT OR REPLACE INTO search_meta(key, value) VALUES('search_schema_version', ?1)",
            params![SEARCH_SCHEMA_VERSION.to_string()],
        )
        .map_err(|error| sqlite_diagnostic("write search schema version", error, path))?;
    Ok(())
}

pub(super) fn sync(
    config: &Config,
    connection: &Connection,
    requested_kinds: &[CatalogKind],
    reindex: bool,
) -> DiagnosticResult<()> {
    let sync_kinds = if reindex {
        CatalogKind::ALL.to_vec()
    } else {
        requested_kinds.to_vec()
    };

    if reindex {
        clear(config, connection)?;
    }

    for kind in &sync_kinds {
        artifact_catalog::refresh_kind(config, *kind)?;
    }
    let records = artifact_catalog::list_records(config, &sync_kinds)?;
    let current = records
        .iter()
        .map(|record| {
            (
                (record.kind.as_str().to_string(), record.id.clone()),
                record,
            )
        })
        .collect::<HashMap<_, _>>();
    let manifest = manifest_for_kinds(config, connection, &sync_kinds)?;
    let fts_keys = fts_keys_for_kinds(config, connection, &sync_kinds)?;

    let transaction = connection.unchecked_transaction().map_err(|error| {
        sqlite_diagnostic("begin search index sync", error, &database_path(config))
    })?;

    for key in fts_keys.iter().filter(|key| !current.contains_key(*key)) {
        delete_document(config, &transaction, &key.0, &key.1)?;
    }
    for (key, _) in manifest
        .iter()
        .filter(|(key, _)| !current.contains_key(*key))
    {
        delete_document(config, &transaction, &key.0, &key.1)?;
    }

    for record in records {
        let key = (record.kind.as_str().to_string(), record.id.clone());
        if manifest
            .get(&key)
            .is_some_and(|freshness| freshness.matches(&record))
            && fts_keys.contains(&key)
        {
            continue;
        }
        let search_document = document::build(config, &record)?;
        upsert_document(config, &transaction, &record, &search_document)?;
    }

    transaction.commit().map_err(|error| {
        sqlite_diagnostic("commit search index sync", error, &database_path(config))
    })?;
    Ok(())
}

fn clear(config: &Config, connection: &Connection) -> DiagnosticResult<()> {
    connection
        .execute_batch(
            "
            DELETE FROM search_fts;
            DELETE FROM search_manifest;
            ",
        )
        .map_err(|error| sqlite_diagnostic("clear search index", error, &database_path(config)))?;
    Ok(())
}

#[derive(Debug)]
struct Freshness {
    path: String,
    mtime_ns: i64,
    size: i64,
    source_hash: String,
}

impl Freshness {
    fn matches(&self, record: &CatalogRecord) -> bool {
        self.path == record.path
            && self.mtime_ns == record.mtime_ns
            && self.size == record.size
            && self.source_hash == record.source_hash
    }
}

fn manifest_for_kinds(
    config: &Config,
    connection: &Connection,
    kinds: &[CatalogKind],
) -> DiagnosticResult<HashMap<(String, String), Freshness>> {
    let mut manifest = HashMap::new();
    for kind in kinds {
        let mut statement = connection
            .prepare(
                "
                SELECT kind, id, path, mtime_ns, size, source_hash
                FROM search_manifest
                WHERE kind = ?1
                ",
            )
            .map_err(|error| {
                sqlite_diagnostic(
                    "prepare search manifest listing",
                    error,
                    &database_path(config),
                )
            })?;
        let rows = statement
            .query_map(params![kind.as_str()], |row| {
                let kind: String = row.get(0)?;
                let id: String = row.get(1)?;
                Ok((
                    (kind, id),
                    Freshness {
                        path: row.get(2)?,
                        mtime_ns: row.get(3)?,
                        size: row.get(4)?,
                        source_hash: row.get(5)?,
                    },
                ))
            })
            .map_err(|error| {
                sqlite_diagnostic(
                    "read search manifest listing",
                    error,
                    &database_path(config),
                )
            })?;
        for row in rows {
            let (key, freshness) = row.map_err(|error| {
                sqlite_diagnostic(
                    "decode search manifest listing",
                    error,
                    &database_path(config),
                )
            })?;
            manifest.insert(key, freshness);
        }
    }
    Ok(manifest)
}

fn fts_keys_for_kinds(
    config: &Config,
    connection: &Connection,
    kinds: &[CatalogKind],
) -> DiagnosticResult<HashSet<(String, String)>> {
    let mut keys = HashSet::new();
    for kind in kinds {
        let mut statement = connection
            .prepare(
                "
                SELECT kind, id
                FROM search_fts
                WHERE kind = ?1
                ",
            )
            .map_err(|error| {
                sqlite_diagnostic("prepare search FTS listing", error, &database_path(config))
            })?;
        let rows = statement
            .query_map(params![kind.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| {
                sqlite_diagnostic("read search FTS listing", error, &database_path(config))
            })?;
        for row in rows {
            keys.insert(row.map_err(|error| {
                sqlite_diagnostic("decode search FTS listing", error, &database_path(config))
            })?);
        }
    }
    Ok(keys)
}

fn delete_document(
    config: &Config,
    connection: &Connection,
    kind: &str,
    id: &str,
) -> DiagnosticResult<()> {
    connection
        .execute(
            "DELETE FROM search_fts WHERE kind = ?1 AND id = ?2",
            params![kind, id],
        )
        .map_err(|error| {
            sqlite_diagnostic(
                "delete stale search document",
                error,
                &database_path(config),
            )
        })?;
    connection
        .execute(
            "DELETE FROM search_manifest WHERE kind = ?1 AND id = ?2",
            params![kind, id],
        )
        .map_err(|error| {
            sqlite_diagnostic(
                "delete stale search manifest entry",
                error,
                &database_path(config),
            )
        })?;
    Ok(())
}

fn upsert_document(
    config: &Config,
    connection: &Connection,
    record: &CatalogRecord,
    document: &SearchDocument,
) -> DiagnosticResult<()> {
    delete_document(config, connection, record.kind.as_str(), &record.id)?;
    connection
        .execute(
            "
            INSERT INTO search_fts(kind, id, title, path, tags, status, body)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
            params![
                document.kind.as_str(),
                document.id,
                document.title,
                document.path,
                encode_tags(&document.tags),
                document.status.clone().unwrap_or_default(),
                document.body
            ],
        )
        .map_err(|error| {
            sqlite_diagnostic("write search document", error, &database_path(config))
        })?;
    connection
        .execute(
            "
            INSERT INTO search_manifest(kind, id, path, mtime_ns, size, source_hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(kind, id) DO UPDATE SET
                path = excluded.path,
                mtime_ns = excluded.mtime_ns,
                size = excluded.size,
                source_hash = excluded.source_hash
            ",
            params![
                record.kind.as_str(),
                record.id,
                record.path,
                record.mtime_ns,
                record.size,
                record.source_hash
            ],
        )
        .map_err(|error| {
            sqlite_diagnostic("write search manifest entry", error, &database_path(config))
        })?;
    Ok(())
}

pub(super) fn encode_tags(tags: &[String]) -> String {
    tags.join("\n")
}
