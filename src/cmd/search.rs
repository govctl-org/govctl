use crate::artifact_catalog::{self, CatalogKind, CatalogRecord};
use crate::cmd::output::{command_table, print_json_array};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::{load_clause, load_rfc};
use crate::model::{AdrEntry, ClauseEntry, GuardEntry, RfcIndex, WorkItemEntry};
use crate::parse::{load_adr, load_guard, load_work_item};
use crate::{ListTarget, OutputFormat};
use comfy_table::{Attribute, Cell};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const SEARCH_SCHEMA_VERSION: i64 = 2;
const DEFAULT_LIMIT: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchResult {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub path: String,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
struct SearchDocument {
    kind: CatalogKind,
    id: String,
    title: String,
    path: String,
    tags: Vec<String>,
    status: Option<String>,
    body: String,
}

#[derive(Debug, Clone)]
struct SearchRow {
    result: SearchResult,
    tags: Vec<String>,
    score: f64,
}

pub fn search(
    config: &Config,
    query: &[String],
    type_filters: &[ListTarget],
    tag_filters: &[String],
    limit: Option<usize>,
    output: OutputFormat,
    reindex: bool,
) -> DiagnosticResult<Diagnostics> {
    let results = search_results(config, query, type_filters, tag_filters, limit, reindex)?;
    print_results(&results, output);
    Ok(vec![])
}

/// Return search results for read-only callers such as the TUI. Implements
/// [[RFC-0007:C-SEARCH]], [[RFC-0007:C-READ-ONLY]], and
/// [[RFC-0002:C-SEARCH-COMMAND]].
pub(crate) fn search_results(
    config: &Config,
    query: &[String],
    type_filters: &[ListTarget],
    tag_filters: &[String],
    limit: Option<usize>,
    reindex: bool,
) -> DiagnosticResult<Vec<SearchResult>> {
    let terms = query_terms(query)?;
    let kinds = kinds_for_filters(type_filters);
    let connection = open_search_index(config)?;
    // Implements [[RFC-0007:C-READ-ONLY]] via disposable derived-index refresh.
    sync_search_index(config, &connection, &kinds, reindex)?;

    // Implements [[RFC-0007:C-SEARCH]] using [[RFC-0002:C-SEARCH-COMMAND]] semantics.
    query_index(
        config,
        &connection,
        &terms,
        &kinds,
        tag_filters,
        limit.unwrap_or(DEFAULT_LIMIT),
    )
}

fn query_terms(query: &[String]) -> DiagnosticResult<Vec<String>> {
    let terms = query
        .iter()
        .flat_map(|arg| arg.split_whitespace())
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "search requires at least one query term",
            "search",
        ));
    }
    Ok(terms)
}

fn kinds_for_filters(type_filters: &[ListTarget]) -> Vec<CatalogKind> {
    if type_filters.is_empty() {
        return vec![
            CatalogKind::Rfc,
            CatalogKind::Clause,
            CatalogKind::Adr,
            CatalogKind::Work,
            CatalogKind::Guard,
        ];
    }

    let mut kinds = Vec::new();
    for filter in type_filters {
        let kind = match filter {
            ListTarget::Rfc => CatalogKind::Rfc,
            ListTarget::Clause => CatalogKind::Clause,
            ListTarget::Adr => CatalogKind::Adr,
            ListTarget::Work => CatalogKind::Work,
            ListTarget::Guard => CatalogKind::Guard,
        };
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    kinds
}

fn open_search_index(config: &Config) -> DiagnosticResult<Connection> {
    let path = artifact_catalog::index_db_path(config);
    let parent = path.parent().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "Local index database path has no parent directory",
            path.display().to_string(),
        )
    })?;
    fs::create_dir_all(parent).map_err(|err| {
        Diagnostic::io_error(
            "create local index directory",
            err,
            parent.display().to_string(),
        )
    })?;
    let connection = Connection::open(&path)
        .map_err(|err| sqlite_diagnostic("open search index", err, &path))?;
    initialize_search_schema(&connection, &path)?;
    Ok(connection)
}

fn initialize_search_schema(connection: &Connection, path: &Path) -> DiagnosticResult<()> {
    connection
        .execute_batch(
            "
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS search_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|err| sqlite_diagnostic("initialize search metadata", err, path))?;

    let version: Option<i64> = connection
        .query_row(
            "SELECT value FROM search_meta WHERE key = 'search_schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| sqlite_diagnostic("read search schema version", err, path))?
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
            .map_err(|err| sqlite_diagnostic("reset search index", err, path))?;
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
        .map_err(|err| sqlite_diagnostic("initialize search index", err, path))?;

    connection
        .execute(
            "INSERT OR REPLACE INTO search_meta(key, value) VALUES('search_schema_version', ?1)",
            params![SEARCH_SCHEMA_VERSION.to_string()],
        )
        .map_err(|err| sqlite_diagnostic("write search schema version", err, path))?;
    Ok(())
}

fn sync_search_index(
    config: &Config,
    connection: &Connection,
    requested_kinds: &[CatalogKind],
    reindex: bool,
) -> DiagnosticResult<()> {
    let sync_kinds = if reindex {
        vec![
            CatalogKind::Rfc,
            CatalogKind::Clause,
            CatalogKind::Adr,
            CatalogKind::Work,
            CatalogKind::Guard,
        ]
    } else {
        requested_kinds.to_vec()
    };

    if reindex {
        // [[RFC-0002:C-SEARCH-COMMAND]]: sync_search_index honors
        // `--reindex` by clearing derived local state with clear_search_index.
        clear_search_index(config, connection)?;
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

    let transaction = connection.unchecked_transaction().map_err(|err| {
        sqlite_diagnostic(
            "begin search index sync",
            err,
            &artifact_catalog::index_db_path(config),
        )
    })?;

    // [[RFC-0002:C-SEARCH-COMMAND]]: sync_search_index treats fts_keys as
    // derived local state; delete_indexed_document removes FTS rows no longer
    // backed by authoritative TOML artifacts.
    for key in fts_keys.iter().filter(|key| !current.contains_key(*key)) {
        delete_indexed_document(config, &transaction, &key.0, &key.1)?;
    }

    // [[RFC-0002:C-SEARCH-COMMAND]]: sync_search_index keeps the manifest in
    // step with authoritative records; delete_indexed_document removes stale
    // manifest rows when their source artifact disappeared.
    for (key, _) in manifest
        .iter()
        .filter(|(key, _)| !current.contains_key(*key))
    {
        delete_indexed_document(config, &transaction, &key.0, &key.1)?;
    }

    for record in records {
        let key = (record.kind.as_str().to_string(), record.id.clone());
        // [[RFC-0002:C-SEARCH-COMMAND]]: freshness is established only when
        // manifest metadata matches and fts_keys still contains the document;
        // otherwise upsert_indexed_document rebuilds the derived index entry.
        if manifest
            .get(&key)
            .is_some_and(|freshness| freshness.matches(&record))
            && fts_keys.contains(&key)
        {
            continue;
        }
        let document = build_search_document(config, &record)?;
        upsert_indexed_document(config, &transaction, &record, &document)?;
    }

    transaction.commit().map_err(|err| {
        sqlite_diagnostic(
            "commit search index sync",
            err,
            &artifact_catalog::index_db_path(config),
        )
    })?;
    Ok(())
}

fn clear_search_index(config: &Config, connection: &Connection) -> DiagnosticResult<()> {
    connection
        .execute_batch(
            "
            DELETE FROM search_fts;
            DELETE FROM search_manifest;
            ",
        )
        .map_err(|err| {
            sqlite_diagnostic(
                "clear search index",
                err,
                &artifact_catalog::index_db_path(config),
            )
        })?;
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
            .map_err(|err| {
                sqlite_diagnostic(
                    "prepare search manifest listing",
                    err,
                    &artifact_catalog::index_db_path(config),
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
            .map_err(|err| {
                sqlite_diagnostic(
                    "read search manifest listing",
                    err,
                    &artifact_catalog::index_db_path(config),
                )
            })?;
        for row in rows {
            let (key, freshness) = row.map_err(|err| {
                sqlite_diagnostic(
                    "decode search manifest listing",
                    err,
                    &artifact_catalog::index_db_path(config),
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
            .map_err(|err| {
                sqlite_diagnostic(
                    "prepare search FTS listing",
                    err,
                    &artifact_catalog::index_db_path(config),
                )
            })?;
        let rows = statement
            .query_map(params![kind.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|err| {
                sqlite_diagnostic(
                    "read search FTS listing",
                    err,
                    &artifact_catalog::index_db_path(config),
                )
            })?;
        for row in rows {
            keys.insert(row.map_err(|err| {
                sqlite_diagnostic(
                    "decode search FTS listing",
                    err,
                    &artifact_catalog::index_db_path(config),
                )
            })?);
        }
    }
    Ok(keys)
}

fn delete_indexed_document(
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
        .map_err(|err| {
            sqlite_diagnostic(
                "delete stale search document",
                err,
                &artifact_catalog::index_db_path(config),
            )
        })?;
    connection
        .execute(
            "DELETE FROM search_manifest WHERE kind = ?1 AND id = ?2",
            params![kind, id],
        )
        .map_err(|err| {
            sqlite_diagnostic(
                "delete stale search manifest entry",
                err,
                &artifact_catalog::index_db_path(config),
            )
        })?;
    Ok(())
}

fn upsert_indexed_document(
    config: &Config,
    connection: &Connection,
    record: &CatalogRecord,
    document: &SearchDocument,
) -> DiagnosticResult<()> {
    delete_indexed_document(config, connection, record.kind.as_str(), &record.id)?;
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
        .map_err(|err| {
            sqlite_diagnostic(
                "write search document",
                err,
                &artifact_catalog::index_db_path(config),
            )
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
        .map_err(|err| {
            sqlite_diagnostic(
                "write search manifest entry",
                err,
                &artifact_catalog::index_db_path(config),
            )
        })?;
    Ok(())
}

fn build_search_document(
    config: &Config,
    record: &CatalogRecord,
) -> DiagnosticResult<SearchDocument> {
    let path = record.absolute_path(config);
    match record.kind {
        CatalogKind::Rfc => {
            let entry = load_rfc(config, &path).map_err(Diagnostic::from)?;
            Ok(rfc_document(config, &entry))
        }
        CatalogKind::Clause => {
            let entry = load_clause(config, &path).map_err(Diagnostic::from)?;
            Ok(clause_document(config, record, &entry))
        }
        CatalogKind::Adr => {
            let entry = load_adr(config, &path)?;
            Ok(adr_document(config, &entry))
        }
        CatalogKind::Work => {
            let entry = load_work_item(config, &path)?;
            Ok(work_document(config, &entry))
        }
        CatalogKind::Guard => {
            let entry = load_guard(config, &path)?;
            Ok(guard_document(config, &entry))
        }
    }
}

fn rfc_document(config: &Config, entry: &RfcIndex) -> SearchDocument {
    let rfc = &entry.rfc;
    let mut parts = vec![
        rfc.rfc_id.clone(),
        rfc.title.clone(),
        rfc.version.clone(),
        rfc.status.as_ref().to_string(),
        rfc.phase.as_ref().to_string(),
    ];
    parts.extend(rfc.owners.iter().cloned());
    parts.extend(rfc.refs.iter().cloned());
    parts.extend(rfc.tags.iter().cloned());
    for section in &rfc.sections {
        parts.push(section.title.clone());
        parts.extend(section.clauses.iter().cloned());
    }
    for changelog in &rfc.changelog {
        parts.push(changelog.version.clone());
        parts.push(changelog.date.clone());
        if let Some(notes) = &changelog.notes {
            parts.push(notes.clone());
        }
        parts.extend(changelog.added.iter().cloned());
        parts.extend(changelog.changed.iter().cloned());
        parts.extend(changelog.deprecated.iter().cloned());
        parts.extend(changelog.removed.iter().cloned());
        parts.extend(changelog.fixed.iter().cloned());
        parts.extend(changelog.security.iter().cloned());
    }
    SearchDocument {
        kind: CatalogKind::Rfc,
        id: rfc.rfc_id.clone(),
        title: rfc.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: rfc.tags.clone(),
        status: Some(format!("{}/{}", rfc.status.as_ref(), rfc.phase.as_ref())),
        body: join_parts(parts),
    }
}

fn clause_document(config: &Config, record: &CatalogRecord, entry: &ClauseEntry) -> SearchDocument {
    let clause = &entry.spec;
    let mut parts = vec![
        record.id.clone(),
        clause.clause_id.clone(),
        clause.title.clone(),
        clause.kind.as_ref().to_string(),
        clause.status.as_ref().to_string(),
        clause.text.clone(),
    ];
    parts.extend(clause.anchors.iter().cloned());
    parts.extend(clause.tags.iter().cloned());
    if let Some(since) = &clause.since {
        parts.push(since.clone());
    }
    if let Some(superseded_by) = &clause.superseded_by {
        parts.push(superseded_by.clone());
    }
    SearchDocument {
        kind: CatalogKind::Clause,
        id: record.id.clone(),
        title: clause.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: clause.tags.clone(),
        status: Some(clause.status.as_ref().to_string()),
        body: join_parts(parts),
    }
}

fn adr_document(config: &Config, entry: &AdrEntry) -> SearchDocument {
    let meta = &entry.spec.govctl;
    let content = &entry.spec.content;
    let mut parts = vec![
        meta.id.clone(),
        meta.title.clone(),
        meta.status.as_ref().to_string(),
        meta.date.clone(),
        content.context.clone(),
        content.decision.clone(),
        content.consequences.clone(),
    ];
    parts.extend(meta.refs.iter().cloned());
    parts.extend(meta.tags.iter().cloned());
    if let Some(superseded_by) = &meta.superseded_by {
        parts.push(superseded_by.clone());
    }
    for alternative in &content.alternatives {
        parts.push(alternative.text.clone());
        parts.push(alternative.status.as_ref().to_string());
        parts.extend(alternative.pros.iter().cloned());
        parts.extend(alternative.cons.iter().cloned());
        if let Some(reason) = &alternative.rejection_reason {
            parts.push(reason.clone());
        }
    }
    SearchDocument {
        kind: CatalogKind::Adr,
        id: meta.id.clone(),
        title: meta.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: meta.tags.clone(),
        status: Some(meta.status.as_ref().to_string()),
        body: join_parts(parts),
    }
}

fn work_document(config: &Config, entry: &WorkItemEntry) -> SearchDocument {
    let meta = &entry.spec.govctl;
    let content = &entry.spec.content;
    let mut parts = vec![
        meta.id.clone(),
        meta.title.clone(),
        meta.status.as_ref().to_string(),
        content.description.clone(),
    ];
    parts.extend(meta.refs.iter().cloned());
    parts.extend(meta.depends_on.iter().cloned());
    parts.extend(meta.tags.iter().cloned());
    for item in &content.acceptance_criteria {
        parts.push(item.text.clone());
        parts.push(item.status.as_ref().to_string());
        parts.push(item.category.as_ref().to_string());
    }
    parts.extend(content.notes.iter().cloned());
    SearchDocument {
        kind: CatalogKind::Work,
        id: meta.id.clone(),
        title: meta.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: meta.tags.clone(),
        status: Some(meta.status.as_ref().to_string()),
        body: join_parts(parts),
    }
}

fn guard_document(config: &Config, entry: &GuardEntry) -> SearchDocument {
    let meta = &entry.spec.govctl;
    let mut parts = vec![
        meta.id.clone(),
        meta.title.clone(),
        entry.spec.check.command.clone(),
    ];
    parts.extend(meta.refs.iter().cloned());
    parts.extend(meta.tags.iter().cloned());
    if let Some(pattern) = &entry.spec.check.pattern {
        parts.push(pattern.clone());
    }
    SearchDocument {
        kind: CatalogKind::Guard,
        id: meta.id.clone(),
        title: meta.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: meta.tags.clone(),
        status: None,
        body: join_parts(parts),
    }
}

fn join_parts(parts: Vec<String>) -> String {
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn query_index(
    config: &Config,
    connection: &Connection,
    terms: &[String],
    kinds: &[CatalogKind],
    tag_filters: &[String],
    limit: usize,
) -> DiagnosticResult<Vec<SearchResult>> {
    let fts_query = fts_query(terms);
    let exact_terms = exact_terms(terms);
    let kind_filter = kinds
        .iter()
        .map(|kind| kind.as_str())
        .collect::<HashSet<_>>();
    let tag_filters = tag_filters
        .iter()
        .map(|tag| tag.as_str())
        .collect::<Vec<_>>();

    let mut statement = connection
        .prepare(
            "
            SELECT kind, id, title, path, tags, status,
                   snippet(search_fts, 6, '', '', '...', 18),
                   bm25(search_fts)
            FROM search_fts
            WHERE search_fts MATCH ?1
            ",
        )
        .map_err(|err| {
            sqlite_diagnostic(
                "prepare search query",
                err,
                &artifact_catalog::index_db_path(config),
            )
        })?;

    let rows = statement
        .query_map(params![fts_query], |row| {
            let tags_blob: String = row.get(4)?;
            let status: String = row.get(5)?;
            let snippet: String = row.get(6)?;
            Ok(SearchRow {
                result: SearchResult {
                    kind: row.get(0)?,
                    id: row.get(1)?,
                    title: row.get(2)?,
                    path: row.get(3)?,
                    snippet: clean_snippet(&snippet),
                    score: Some(row.get(7)?),
                    status: (!status.is_empty()).then_some(status),
                },
                tags: decode_tags(&tags_blob),
                score: row.get(7)?,
            })
        })
        .map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!("Invalid search query: {err}"),
                "search",
            )
        })?;

    let mut matches = Vec::new();
    for row in rows {
        let row = row.map_err(|err| {
            sqlite_diagnostic(
                "read search result",
                err,
                &artifact_catalog::index_db_path(config),
            )
        })?;
        if !kind_filter.contains(row.result.kind.as_str()) {
            continue;
        }
        if !has_all_tags(&row.tags, &tag_filters) {
            continue;
        }
        matches.push(row);
    }

    matches.sort_by(|a, b| {
        let a_exact = exact_terms.contains(&a.result.id.to_lowercase());
        let b_exact = exact_terms.contains(&b.result.id.to_lowercase());
        match (a_exact, b_exact) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a
                .score
                .partial_cmp(&b.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.result.id.cmp(&b.result.id)),
        }
    });

    Ok(matches
        .into_iter()
        .take(limit)
        .map(|row| row.result)
        .collect())
}

fn fts_query(terms: &[String]) -> String {
    terms
        .iter()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn exact_terms(terms: &[String]) -> HashSet<String> {
    let mut exact = terms
        .iter()
        .map(|term| term.to_lowercase())
        .collect::<HashSet<_>>();
    exact.insert(terms.join(" ").to_lowercase());
    exact
}

fn encode_tags(tags: &[String]) -> String {
    tags.join("\n")
}

fn decode_tags(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_all_tags(tags: &[String], filters: &[&str]) -> bool {
    filters
        .iter()
        .all(|filter| tags.iter().any(|tag| tag == filter))
}

fn clean_snippet(snippet: &str) -> String {
    snippet.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn print_results(results: &[SearchResult], output: OutputFormat) {
    match output {
        OutputFormat::Json => print_json_array(results),
        OutputFormat::Plain => {
            for result in results {
                println!("{}", result.id);
            }
        }
        OutputFormat::Table => {
            let mut table = command_table();
            table.set_header(
                ["Kind", "ID", "Title", "Path", "Snippet"]
                    .iter()
                    .map(|header| Cell::new(*header).add_attribute(Attribute::Bold))
                    .collect::<Vec<_>>(),
            );
            for result in results {
                table.add_row(vec![
                    Cell::new(&result.kind),
                    Cell::new(&result.id),
                    Cell::new(&result.title),
                    Cell::new(&result.path),
                    Cell::new(&result.snippet),
                ]);
            }
            println!("{table}");
        }
    }
}

fn sqlite_diagnostic(action: &'static str, err: rusqlite::Error, path: &Path) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0903UnexpectedError,
        format!("{action}: {err}"),
        path.display().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_terms_rejects_empty_input_and_splits_whitespace()
    -> Result<(), Box<dyn std::error::Error>> {
        let empty = match query_terms(&[]) {
            Ok(_) => return Err(std::io::Error::other("empty search returned Ok").into()),
            Err(err) => err,
        };
        assert_eq!(empty.code, DiagnosticCode::E0801MissingRequiredArg);

        let terms = match query_terms(&["  cache  index ".to_string(), "ADR-0001".to_string()]) {
            Ok(terms) => terms,
            Err(err) => {
                return Err(
                    std::io::Error::other(format!("non-empty search failed: {err:?}")).into(),
                );
            }
        };
        assert_eq!(terms, vec!["cache", "index", "ADR-0001"]);
        Ok(())
    }

    #[test]
    fn kind_filters_map_all_targets_and_deduplicate() {
        let kinds = kinds_for_filters(&[
            ListTarget::Rfc,
            ListTarget::Clause,
            ListTarget::Adr,
            ListTarget::Work,
            ListTarget::Guard,
            ListTarget::Adr,
        ]);
        assert_eq!(
            kinds,
            vec![
                CatalogKind::Rfc,
                CatalogKind::Clause,
                CatalogKind::Adr,
                CatalogKind::Work,
                CatalogKind::Guard
            ]
        );
    }

    #[test]
    fn fts_query_quotes_user_terms_and_exact_terms_include_phrase() {
        let terms = vec!["cache".to_string(), "quote\"term".to_string()];
        assert_eq!(fts_query(&terms), "\"cache\" \"quote\"\"term\"");

        let exact = exact_terms(&terms);
        assert!(exact.contains("cache"));
        assert!(exact.contains("quote\"term"));
        assert!(exact.contains("cache quote\"term"));
    }

    #[test]
    fn tag_and_snippet_helpers_normalize_values() {
        let tags = vec!["search".to_string(), "cache".to_string()];
        let encoded = encode_tags(&tags);
        assert_eq!(decode_tags(&format!("\n{encoded}\n\n")), tags);
        assert!(has_all_tags(&tags, &["search", "cache"]));
        assert!(!has_all_tags(&tags, &["search", "missing"]));
        assert_eq!(
            clean_snippet(" cache\n\nresult\tbody "),
            "cache result body"
        );
    }
}
