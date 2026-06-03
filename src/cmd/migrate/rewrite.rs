use super::ops::FileOp;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult};
use crate::schema::{ArtifactSchema, with_schema_header};
use std::fs;
use std::path::{Path, PathBuf};

/// Strip `schema = N` lines from a `[govctl]` section in raw TOML text.
fn strip_govctl_schema(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    let mut in_govctl = false;
    lines.retain(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_govctl = trimmed == "[govctl]";
        }
        !(in_govctl && trimmed.starts_with("schema") && trimmed.contains('='))
    });
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Check if a TOML file needs rewrite (missing header or has `govctl.schema`).
fn needs_rewrite(content: &str) -> bool {
    if !content.starts_with("#:schema ") {
        return true;
    }
    let mut in_govctl = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_govctl = trimmed == "[govctl]";
        }
        if in_govctl && trimmed.starts_with("schema") && trimmed.contains('=') {
            return true;
        }
    }
    false
}

/// Rewrite a TOML file: ensure `#:schema` header and strip `govctl.schema`.
fn rewrite_toml(content: &str, schema: ArtifactSchema) -> String {
    let cleaned = strip_govctl_schema(content);
    if cleaned.starts_with("#:schema ") {
        cleaned
    } else {
        with_schema_header(schema, &cleaned)
    }
}

fn rewrite_file_op(path: &Path, schema: ArtifactSchema) -> Option<FileOp> {
    let content = fs::read_to_string(path).ok()?;
    if !needs_rewrite(&content) {
        return None;
    }
    Some(FileOp::Write {
        path: path.to_path_buf(),
        content: rewrite_toml(&content, schema),
    })
}

/// Collect TOML files in a directory that need rewriting.
fn collect_rewrites(dir: &Path, schema: ArtifactSchema) -> Vec<FileOp> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut ops: Vec<(PathBuf, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if let Some(FileOp::Write { path, content }) = rewrite_file_op(&path, schema) {
            ops.push((path, content));
        }
    }
    ops.sort_by(|a, b| a.0.cmp(&b.0));
    ops.into_iter()
        .map(|(path, content)| FileOp::Write { path, content })
        .collect()
}

/// Plan header + schema-strip rewrites for all TOML artifacts.
pub(super) fn plan_toml_rewrites(
    config: &Config,
    skip_releases: bool,
) -> DiagnosticResult<Vec<FileOp>> {
    let mut ops = Vec::new();

    ops.extend(collect_rewrites(&config.adr_dir(), ArtifactSchema::Adr));
    ops.extend(collect_rewrites(
        &config.work_dir(),
        ArtifactSchema::WorkItem,
    ));
    ops.extend(collect_rewrites(&config.guard_dir(), ArtifactSchema::Guard));

    let rfc_root = config.rfc_dir();
    if rfc_root.exists() {
        for entry in fs::read_dir(&rfc_root)
            .map_err(|err| {
                Diagnostic::io_error(
                    "read RFC directory for TOML rewrites",
                    err,
                    config.display_path(&rfc_root).display().to_string(),
                )
            })?
            .flatten()
        {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let rfc_toml = dir.join("rfc.toml");
            if let Some(op) = rewrite_file_op(&rfc_toml, ArtifactSchema::Rfc) {
                ops.push(op);
            }
            ops.extend(collect_rewrites(
                &dir.join("clauses"),
                ArtifactSchema::Clause,
            ));
        }
    }

    if !skip_releases {
        let releases_path = config.releases_path();
        if let Some(op) = rewrite_file_op(&releases_path, ArtifactSchema::Release) {
            ops.push(op);
        }
    }

    Ok(ops)
}
