use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn project_root(config: &Config) -> &Path {
    config
        .gov_root
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

pub(crate) fn database_path(config: &Config) -> PathBuf {
    project_root(config).join(".govctl").join("index.db")
}

pub(crate) fn open_database(
    config: &Config,
    action: &'static str,
) -> DiagnosticResult<(Connection, PathBuf)> {
    let path = database_path(config);
    let parent = path.parent().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "Local index database path has no parent directory",
            path.display().to_string(),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        Diagnostic::io_error(
            "create local index directory",
            error,
            parent.display().to_string(),
        )
    })?;
    let connection =
        Connection::open(&path).map_err(|error| sqlite_diagnostic(action, error, &path))?;
    connection
        .execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|error| sqlite_diagnostic("configure local index database", error, &path))?;
    Ok((connection, path))
}

pub(crate) fn sqlite_diagnostic(
    action: &'static str,
    error: rusqlite::Error,
    path: &Path,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0903UnexpectedError,
        format!("{action}: {error}"),
        path.display().to_string(),
    )
}
