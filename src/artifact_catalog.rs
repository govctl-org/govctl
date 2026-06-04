use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{AdrEntry, GuardEntry, WorkItemEntry};
use crate::parse::{load_adr, load_adrs, load_guard, load_guards, load_work_item, load_work_items};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const CATALOG_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CatalogKind {
    Rfc,
    Clause,
    Adr,
    Work,
    Guard,
}

impl CatalogKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Rfc => "rfc",
            Self::Clause => "clause",
            Self::Adr => "adr",
            Self::Work => "work",
            Self::Guard => "guard",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "rfc" => Some(Self::Rfc),
            "clause" => Some(Self::Clause),
            "adr" => Some(Self::Adr),
            "work" => Some(Self::Work),
            "guard" => Some(Self::Guard),
            _ => None,
        }
    }

    fn dir(self, config: &Config) -> PathBuf {
        match self {
            Self::Rfc | Self::Clause => config.rfc_dir(),
            Self::Adr => config.adr_dir(),
            Self::Work => config.work_dir(),
            Self::Guard => config.guard_dir(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CatalogRecord {
    pub(crate) kind: CatalogKind,
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) mtime_ns: i64,
    pub(crate) size: i64,
    pub(crate) source_hash: String,
}

#[derive(Debug, Clone, Copy)]
struct MissingArtifact {
    code: DiagnosticCode,
    label: &'static str,
}

impl CatalogRecord {
    pub(crate) fn absolute_path(&self, config: &Config) -> PathBuf {
        project_root(config).join(&self.path)
    }
}

pub(crate) fn load_adr_by_id(config: &Config, id: &str) -> DiagnosticResult<AdrEntry> {
    load_cataloged_entry(
        config,
        CatalogKind::Adr,
        id,
        load_adr,
        load_adrs,
        |entry| &entry.spec.govctl.id,
        MissingArtifact {
            code: DiagnosticCode::E0302AdrNotFound,
            label: "ADR",
        },
    )
}

pub(crate) fn load_work_item_by_id(config: &Config, id: &str) -> DiagnosticResult<WorkItemEntry> {
    load_cataloged_entry(
        config,
        CatalogKind::Work,
        id,
        load_work_item,
        load_work_items,
        |entry| &entry.spec.govctl.id,
        MissingArtifact {
            code: DiagnosticCode::E0402WorkNotFound,
            label: "Work item",
        },
    )
}

pub(crate) fn load_guard_by_id(config: &Config, id: &str) -> DiagnosticResult<GuardEntry> {
    load_cataloged_entry(
        config,
        CatalogKind::Guard,
        id,
        load_guard,
        load_guards,
        |entry| &entry.spec.govctl.id,
        MissingArtifact {
            code: DiagnosticCode::E1002GuardNotFound,
            label: "Guard",
        },
    )
}

fn load_cataloged_entry<T>(
    config: &Config,
    kind: CatalogKind,
    id: &str,
    load_one: fn(&Config, &Path) -> DiagnosticResult<T>,
    load_all: fn(&Config) -> DiagnosticResult<Vec<T>>,
    entry_id: impl Fn(&T) -> &String,
    missing: MissingArtifact,
) -> DiagnosticResult<T> {
    if let Some(entry) = load_from_catalog(config, kind, id, load_one, &entry_id) {
        return Ok(entry);
    }

    let entries = load_all(config)?;
    let found = entries.into_iter().find(|entry| entry_id(entry) == id);
    let Some(entry) = found else {
        let location = config.display_path(&kind.dir(config)).display().to_string();
        return Err(Diagnostic::new(
            missing.code,
            format!("{} not found: {id}", missing.label),
            location,
        ));
    };

    // [[RFC-0002:C-SEARCH-COMMAND]]: TOML artifacts are authoritative; refresh
    // the derived `.govctl/index.db` catalog opportunistically after fallback.
    let _ = refresh_kind(config, kind);
    Ok(entry)
}

fn load_from_catalog<T>(
    config: &Config,
    kind: CatalogKind,
    id: &str,
    load_one: fn(&Config, &Path) -> DiagnosticResult<T>,
    entry_id: &impl Fn(&T) -> &String,
) -> Option<T> {
    // [[RFC-0002:C-SEARCH-COMMAND]]: cached_path reads the derived local
    // `.govctl/index.db` catalog, never an authoritative artifact source.
    let path = cached_path(config, kind, id)?;
    let entry = load_one(config, &path).ok()?;
    // [[RFC-0002:C-RESOURCES]]: stable resource IDs are authoritative; reject a
    // cached path whose loaded TOML entry carries a different ID.
    if entry_id(&entry) == id {
        return Some(entry);
    }

    // [[RFC-0002:C-SEARCH-COMMAND]]: repair stale derived catalog state from
    // authoritative TOML artifacts before retrying cached_path.
    let _ = refresh_kind(config, kind);
    // [[RFC-0002:C-SEARCH-COMMAND]]: retry against the refreshed
    // `.govctl/index.db` catalog after the repair pass.
    let path = cached_path(config, kind, id)?;
    let entry = load_one(config, &path).ok()?;
    // [[RFC-0002:C-RESOURCES]]: the repaired cached path is accepted only when
    // the loaded TOML entry ID matches the requested resource ID.
    (entry_id(&entry) == id).then_some(entry)
}

fn cached_path(config: &Config, kind: CatalogKind, id: &str) -> Option<PathBuf> {
    try_cached_path(config, kind, id).ok().flatten()
}

fn try_cached_path(
    config: &Config,
    kind: CatalogKind,
    id: &str,
) -> DiagnosticResult<Option<PathBuf>> {
    let connection = open_catalog(config)?;
    if let Some(path) = lookup_path(config, &connection, kind, id)?
        && path.exists()
    {
        return Ok(Some(path));
    }

    refresh_kind_with_connection(config, &connection, kind)?;
    lookup_path(config, &connection, kind, id)
}

pub(crate) fn refresh_kind(config: &Config, kind: CatalogKind) -> DiagnosticResult<()> {
    let connection = open_catalog(config)?;
    refresh_kind_with_connection(config, &connection, kind)
}

pub(crate) fn list_records(
    config: &Config,
    kinds: &[CatalogKind],
) -> DiagnosticResult<Vec<CatalogRecord>> {
    let connection = open_catalog(config)?;
    let mut records = Vec::new();

    for kind in kinds {
        let requested_kind = *kind;
        let mut statement = connection
            .prepare(
                "
                SELECT kind, id, path, mtime_ns, size, source_hash
                FROM artifact_catalog
                WHERE kind = ?1
                ORDER BY id
                ",
            )
            .map_err(|err| {
                sqlite_diagnostic(
                    "prepare local artifact catalog listing",
                    err,
                    &index_db_path(config),
                )
            })?;
        let rows = statement
            .query_map(params![kind.as_str()], |row| {
                let kind_value: String = row.get(0)?;
                Ok(CatalogRecord {
                    kind: CatalogKind::from_str(&kind_value).unwrap_or(requested_kind),
                    id: row.get(1)?,
                    path: row.get(2)?,
                    mtime_ns: row.get(3)?,
                    size: row.get(4)?,
                    source_hash: row.get(5)?,
                })
            })
            .map_err(|err| {
                sqlite_diagnostic(
                    "read local artifact catalog listing",
                    err,
                    &index_db_path(config),
                )
            })?;

        for row in rows {
            records.push(row.map_err(|err| {
                sqlite_diagnostic(
                    "decode local artifact catalog listing",
                    err,
                    &index_db_path(config),
                )
            })?);
        }
    }

    Ok(records)
}

fn open_catalog(config: &Config) -> DiagnosticResult<Connection> {
    let path = index_db_path(config);
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
        .map_err(|err| sqlite_diagnostic("open local artifact catalog", err, &path))?;
    initialize_schema(&connection, &path)?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection, path: &Path) -> DiagnosticResult<()> {
    connection
        .execute_batch(
            "
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS catalog_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS artifact_catalog (
                kind TEXT NOT NULL,
                id TEXT NOT NULL,
                path TEXT NOT NULL,
                mtime_ns INTEGER NOT NULL,
                size INTEGER NOT NULL,
                source_hash TEXT NOT NULL,
                PRIMARY KEY (kind, id)
            );
            CREATE INDEX IF NOT EXISTS idx_artifact_catalog_path
                ON artifact_catalog(path);
            ",
        )
        .map_err(|err| sqlite_diagnostic("initialize local artifact catalog", err, path))?;

    let version: Option<i64> = connection
        .query_row(
            "SELECT value FROM catalog_meta WHERE key = 'catalog_schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| sqlite_diagnostic("read local artifact catalog version", err, path))?
        .and_then(|value| value.parse::<i64>().ok());

    if version != Some(CATALOG_SCHEMA_VERSION) {
        connection
            .execute_batch(
                "
                DELETE FROM artifact_catalog;
                DELETE FROM catalog_meta WHERE key = 'catalog_schema_version';
                ",
            )
            .map_err(|err| sqlite_diagnostic("reset local artifact catalog", err, path))?;
    }

    connection
        .execute(
            "INSERT OR REPLACE INTO catalog_meta(key, value) VALUES('catalog_schema_version', ?1)",
            params![CATALOG_SCHEMA_VERSION.to_string()],
        )
        .map_err(|err| sqlite_diagnostic("write local artifact catalog version", err, path))?;
    Ok(())
}

fn refresh_kind_with_connection(
    config: &Config,
    connection: &Connection,
    kind: CatalogKind,
) -> DiagnosticResult<()> {
    let entries = scan_kind(config, kind)?;
    let transaction = connection.unchecked_transaction().map_err(|err| {
        sqlite_diagnostic(
            "begin local artifact catalog refresh",
            err,
            &index_db_path(config),
        )
    })?;
    transaction
        .execute(
            "DELETE FROM artifact_catalog WHERE kind = ?1",
            params![kind.as_str()],
        )
        .map_err(|err| {
            sqlite_diagnostic(
                "clear local artifact catalog entries",
                err,
                &index_db_path(config),
            )
        })?;

    for entry in entries {
        transaction
            .execute(
                "
                INSERT OR REPLACE INTO artifact_catalog
                    (kind, id, path, mtime_ns, size, source_hash)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    kind.as_str(),
                    entry.id,
                    entry.path,
                    entry.mtime_ns,
                    entry.size,
                    entry.source_hash
                ],
            )
            .map_err(|err| {
                sqlite_diagnostic(
                    "write local artifact catalog entry",
                    err,
                    &index_db_path(config),
                )
            })?;
    }

    transaction.commit().map_err(|err| {
        sqlite_diagnostic(
            "commit local artifact catalog refresh",
            err,
            &index_db_path(config),
        )
    })?;
    Ok(())
}

fn lookup_path(
    config: &Config,
    connection: &Connection,
    kind: CatalogKind,
    id: &str,
) -> DiagnosticResult<Option<PathBuf>> {
    let path: Option<String> = connection
        .query_row(
            "SELECT path FROM artifact_catalog WHERE kind = ?1 AND id = ?2",
            params![kind.as_str(), id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| {
            sqlite_diagnostic(
                "read local artifact catalog entry",
                err,
                &index_db_path(config),
            )
        })?;
    Ok(path.map(|path| project_root(config).join(path)))
}

#[derive(Debug)]
struct CatalogEntry {
    id: String,
    path: String,
    mtime_ns: i64,
    size: i64,
    source_hash: String,
}

fn scan_kind(config: &Config, kind: CatalogKind) -> DiagnosticResult<Vec<CatalogEntry>> {
    match kind {
        CatalogKind::Rfc => return scan_rfc_kind(config),
        CatalogKind::Clause => return scan_clause_kind(config),
        CatalogKind::Adr | CatalogKind::Work | CatalogKind::Guard => {}
    }

    let dir = kind.dir(config);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(&dir).map_err(|err| {
        Diagnostic::io_error("read artifact directory", err, dir.display().to_string())
    })?;
    let mut catalog_entries = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::io_error(
                "read artifact directory entry",
                err,
                dir.display().to_string(),
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        if let Some(catalog_entry) = read_catalog_entry(config, &path)? {
            catalog_entries.push(catalog_entry);
        }
    }

    catalog_entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(catalog_entries)
}

fn scan_rfc_kind(config: &Config) -> DiagnosticResult<Vec<CatalogEntry>> {
    let dir = config.rfc_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut catalog_entries = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|err| {
        Diagnostic::io_error("read RFC directory", err, dir.display().to_string())
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::io_error("read RFC directory entry", err, dir.display().to_string())
        })?;
        let path = entry.path().join("rfc.toml");
        if path.exists()
            && let Some(catalog_entry) = read_catalog_entry(config, &path)?
        {
            catalog_entries.push(catalog_entry);
        }
    }

    catalog_entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(catalog_entries)
}

fn scan_clause_kind(config: &Config) -> DiagnosticResult<Vec<CatalogEntry>> {
    let dir = config.rfc_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut catalog_entries = Vec::new();
    let rfc_dirs = fs::read_dir(&dir).map_err(|err| {
        Diagnostic::io_error("read RFC directory", err, dir.display().to_string())
    })?;
    for rfc_dir in rfc_dirs {
        let rfc_dir = rfc_dir.map_err(|err| {
            Diagnostic::io_error("read RFC directory entry", err, dir.display().to_string())
        })?;
        let rfc_path = rfc_dir.path();
        if !rfc_path.is_dir() {
            continue;
        }
        let Some(rfc_id) = rfc_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let clauses_dir = rfc_path.join("clauses");
        if !clauses_dir.exists() {
            continue;
        }

        let clauses = fs::read_dir(&clauses_dir).map_err(|err| {
            Diagnostic::io_error(
                "read clause directory",
                err,
                clauses_dir.display().to_string(),
            )
        })?;
        for clause in clauses {
            let clause = clause.map_err(|err| {
                Diagnostic::io_error(
                    "read clause directory entry",
                    err,
                    clauses_dir.display().to_string(),
                )
            })?;
            let path = clause.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                continue;
            }
            if let Some(mut catalog_entry) = read_catalog_entry(config, &path)? {
                catalog_entry.id = format!("{rfc_id}:{}", catalog_entry.id);
                catalog_entries.push(catalog_entry);
            }
        }
    }

    catalog_entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(catalog_entries)
}

fn read_catalog_entry(config: &Config, path: &Path) -> DiagnosticResult<Option<CatalogEntry>> {
    let content = fs::read_to_string(path).map_err(|err| {
        Diagnostic::io_error("read artifact for catalog", err, path.display().to_string())
    })?;
    let raw: toml::Value = match toml::from_str(&content) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(id) = raw
        .get("govctl")
        .and_then(|govctl| govctl.get("id"))
        .and_then(toml::Value::as_str)
    else {
        return Ok(None);
    };

    let metadata = fs::metadata(path).map_err(|err| {
        Diagnostic::io_error("read artifact metadata", err, path.display().to_string())
    })?;
    Ok(Some(CatalogEntry {
        id: id.to_string(),
        path: config.display_path(path).display().to_string(),
        mtime_ns: mtime_ns(&metadata),
        size: i64::try_from(metadata.len()).unwrap_or(i64::MAX),
        source_hash: sha256_hex(&content),
    }))
}

fn mtime_ns(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn index_db_path(config: &Config) -> PathBuf {
    project_root(config).join(".govctl").join("index.db")
}

pub(crate) fn project_root(config: &Config) -> &Path {
    config
        .gov_root
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
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

    fn test_config(root: &Path) -> Config {
        Config {
            gov_root: root.join("gov"),
            ..Config::default()
        }
    }

    fn write_work_file(path: &Path, id: &str) -> std::io::Result<()> {
        fs::write(
            path,
            format!(
                r#"[govctl]
id = "{id}"
title = "Test"
status = "queue"
created = "2026-06-04"

[content]
description = "Test"
"#
            ),
        )
    }

    fn write_adr_file(path: &Path, id: &str) -> std::io::Result<()> {
        fs::write(
            path,
            format!(
                r#"[govctl]
id = "{id}"
title = "Catalog ADR"
status = "accepted"
date = "2026-06-04"

[content]
context = "Catalog context"
decision = "Catalog decision"
consequences = "Catalog consequences"
"#
            ),
        )
    }

    fn write_guard_file(path: &Path, id: &str) -> std::io::Result<()> {
        fs::write(
            path,
            format!(
                r#"[govctl]
id = "{id}"
title = "Catalog Guard"

[check]
command = "echo catalog"
"#
            ),
        )
    }

    fn write_rfc_file(path: &Path, id: &str) -> std::io::Result<()> {
        fs::write(
            path,
            format!(
                r#"[govctl]
id = "{id}"
title = "Catalog RFC"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-06-04"

[[sections]]
title = "Specification"
clauses = ["clauses/C-CATALOG.toml"]
"#
            ),
        )
    }

    fn write_clause_file(path: &Path, id: &str) -> std::io::Result<()> {
        fs::write(
            path,
            format!(
                r#"[govctl]
id = "{id}"
title = "Catalog Clause"
kind = "normative"
status = "active"

[content]
text = "Catalog clause"
"#
            ),
        )
    }

    #[test]
    fn catalog_kind_helpers_handle_unknown_values_and_rfc_dirs()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());

        assert!(CatalogKind::from_str("unknown").is_none());
        assert_eq!(CatalogKind::Rfc.dir(&config), config.rfc_dir());
        assert_eq!(CatalogKind::Clause.dir(&config), config.rfc_dir());

        let record = CatalogRecord {
            kind: CatalogKind::Work,
            id: "WI-2026-06-04-994".to_string(),
            path: "gov/work/item.toml".to_string(),
            mtime_ns: 0,
            size: 0,
            source_hash: String::new(),
        };
        assert_eq!(
            record.absolute_path(&config),
            temp.path().join("gov/work/item.toml")
        );
        Ok(())
    }

    #[test]
    fn scans_return_empty_when_artifact_dirs_are_missing() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());

        assert!(scan_kind(&config, CatalogKind::Work)?.is_empty());
        assert!(scan_kind(&config, CatalogKind::Rfc)?.is_empty());
        assert!(scan_kind(&config, CatalogKind::Clause)?.is_empty());
        Ok(())
    }

    #[test]
    fn schema_version_reset_clears_stale_catalog_rows() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());
        let path = index_db_path(&config);
        let connection = open_catalog(&config)?;
        connection.execute(
            "UPDATE catalog_meta SET value = '0' WHERE key = 'catalog_schema_version'",
            [],
        )?;
        connection.execute(
            "
            INSERT INTO artifact_catalog(kind, id, path, mtime_ns, size, source_hash)
            VALUES('work', 'WI-2026-06-04-993', 'gov/work/stale.toml', 0, 0, '')
            ",
            [],
        )?;

        initialize_schema(&connection, &path)?;

        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM artifact_catalog", [], |row| {
                row.get(0)
            })?;
        assert_eq!(count, 0);

        connection.execute(
            "UPDATE catalog_meta SET value = 'not-a-number' WHERE key = 'catalog_schema_version'",
            [],
        )?;
        connection.execute(
            "
            INSERT INTO artifact_catalog(kind, id, path, mtime_ns, size, source_hash)
            VALUES('work', 'WI-2026-06-04-992', 'gov/work/malformed.toml', 0, 0, '')
            ",
            [],
        )?;

        initialize_schema(&connection, &path)?;

        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM artifact_catalog", [], |row| {
                row.get(0)
            })?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn read_catalog_entry_reports_missing_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());
        let err = match read_catalog_entry(&config, &temp.path().join("missing.toml")) {
            Ok(_) => return Err(std::io::Error::other("missing file returned Ok").into()),
            Err(err) => err,
        };

        assert_eq!(err.code, DiagnosticCode::E0901IoError);
        Ok(())
    }

    #[test]
    fn cached_path_repairs_missing_entry_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());
        let work_dir = config.work_dir();
        fs::create_dir_all(&work_dir)?;
        let first_path = work_dir.join("first.toml");
        let second_path = work_dir.join("second.toml");
        write_work_file(&first_path, "WI-2026-06-04-999")?;

        refresh_kind(&config, CatalogKind::Work)?;
        assert_eq!(
            cached_path(&config, CatalogKind::Work, "WI-2026-06-04-999"),
            Some(first_path.clone())
        );

        fs::remove_file(first_path)?;
        write_work_file(&second_path, "WI-2026-06-04-999")?;

        assert_eq!(
            cached_path(&config, CatalogKind::Work, "WI-2026-06-04-999"),
            Some(second_path)
        );
        Ok(())
    }

    #[test]
    fn loaders_repair_cached_path_with_mismatched_id() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());
        let work_dir = config.work_dir();
        fs::create_dir_all(&work_dir)?;
        let stale_path = work_dir.join("stale.toml");
        let repaired_path = work_dir.join("repaired.toml");
        write_work_file(&stale_path, "WI-2026-06-04-997")?;

        refresh_kind(&config, CatalogKind::Work)?;
        write_work_file(&stale_path, "WI-2026-06-04-996")?;
        write_work_file(&repaired_path, "WI-2026-06-04-997")?;

        let item = load_work_item_by_id(&config, "WI-2026-06-04-997")?;
        assert_eq!(item.path, repaired_path);
        Ok(())
    }

    #[test]
    fn loader_reports_missing_artifact() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());
        fs::create_dir_all(config.work_dir())?;

        let err = match load_work_item_by_id(&config, "WI-2026-06-04-000") {
            Ok(_) => return Err(std::io::Error::other("missing work item returned Ok").into()),
            Err(err) => err,
        };
        assert_eq!(err.code, DiagnosticCode::E0402WorkNotFound);
        assert!(err.message.contains("Work item not found"));
        Ok(())
    }

    #[test]
    fn refresh_indexes_all_artifact_kinds_and_skips_noise() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());
        fs::create_dir_all(config.work_dir())?;
        fs::create_dir_all(config.adr_dir())?;
        fs::create_dir_all(config.guard_dir())?;
        fs::create_dir_all(config.rfc_dir().join("RFC-0009/clauses"))?;

        write_work_file(
            &config.work_dir().join("catalog-work.toml"),
            "WI-2026-06-04-995",
        )?;
        write_adr_file(&config.adr_dir().join("ADR-0099-catalog.toml"), "ADR-0099")?;
        write_guard_file(
            &config.guard_dir().join("guard-catalog.toml"),
            "GUARD-CATALOG",
        )?;
        write_rfc_file(&config.rfc_dir().join("RFC-0009/rfc.toml"), "RFC-0009")?;
        write_clause_file(
            &config.rfc_dir().join("RFC-0009/clauses/C-CATALOG.toml"),
            "C-CATALOG",
        )?;
        fs::write(config.work_dir().join("README.md"), "not an artifact")?;
        fs::write(config.work_dir().join("invalid.toml"), "not = [")?;
        fs::write(config.work_dir().join("missing-id.toml"), "[content]\n")?;

        for kind in [
            CatalogKind::Rfc,
            CatalogKind::Clause,
            CatalogKind::Adr,
            CatalogKind::Work,
            CatalogKind::Guard,
        ] {
            refresh_kind(&config, kind)?;
        }

        let records = list_records(
            &config,
            &[
                CatalogKind::Rfc,
                CatalogKind::Clause,
                CatalogKind::Adr,
                CatalogKind::Work,
                CatalogKind::Guard,
            ],
        )?;
        let ids = records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"RFC-0009"));
        assert!(ids.contains(&"RFC-0009:C-CATALOG"));
        assert!(ids.contains(&"ADR-0099"));
        assert!(ids.contains(&"WI-2026-06-04-995"));
        assert!(ids.contains(&"GUARD-CATALOG"));

        assert_eq!(
            load_adr_by_id(&config, "ADR-0099")?.spec.govctl.title,
            "Catalog ADR"
        );
        assert_eq!(
            load_guard_by_id(&config, "GUARD-CATALOG")?
                .spec
                .check
                .command,
            "echo catalog"
        );
        Ok(())
    }

    #[test]
    fn refresh_removes_deleted_catalog_entries() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config = test_config(temp.path());
        let work_dir = config.work_dir();
        fs::create_dir_all(&work_dir)?;
        let path = work_dir.join("work.toml");
        write_work_file(&path, "WI-2026-06-04-998")?;

        refresh_kind(&config, CatalogKind::Work)?;
        assert!(cached_path(&config, CatalogKind::Work, "WI-2026-06-04-998").is_some());

        fs::remove_file(path)?;
        refresh_kind(&config, CatalogKind::Work)?;

        assert!(cached_path(&config, CatalogKind::Work, "WI-2026-06-04-998").is_none());
        Ok(())
    }
}
