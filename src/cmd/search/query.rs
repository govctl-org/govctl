use super::SearchResult;
use crate::artifact_catalog::CatalogKind;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::local_index::{database_path, sqlite_diagnostic};
use rusqlite::{Connection, params};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub(super) struct SearchRow {
    pub(super) result: SearchResult,
    pub(super) tags: Vec<String>,
    pub(super) score: f64,
}

pub(super) fn query_index(
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
    let tag_filters = tag_filters.iter().map(String::as_str).collect::<Vec<_>>();

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
        .map_err(|error| {
            sqlite_diagnostic("prepare search query", error, &database_path(config))
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
                    scenario_path: None,
                },
                tags: decode_tags(&tags_blob),
                score: row.get(7)?,
            })
        })
        .map_err(|error| {
            Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!("Invalid search query: {error}"),
                "search",
            )
        })?;

    let mut matches = Vec::new();
    for row in rows {
        let row = row.map_err(|error| {
            sqlite_diagnostic("read search result", error, &database_path(config))
        })?;
        if kind_filter.contains(row.result.kind.as_str()) && has_all_tags(&row.tags, &tag_filters) {
            matches.push(row);
        }
    }

    matches.sort_by(|left, right| compare_search_rows(left, right, &exact_terms));
    let mut results = matches
        .into_iter()
        .take(limit)
        .map(|row| row.result)
        .collect::<Vec<_>>();
    populate_scenario_paths(config, &mut results)?;
    Ok(results)
}

fn populate_scenario_paths(config: &Config, results: &mut [SearchResult]) -> DiagnosticResult<()> {
    if !results.iter().any(|result| result.kind == "conformance") {
        return Ok(());
    }
    let scenario_paths = crate::parse::load_conformance_cases(config)?
        .into_iter()
        .map(|entry| (entry.meta().id.clone(), entry.spec.case.path))
        .collect::<HashMap<_, _>>();
    for result in results {
        if result.kind == "conformance" {
            result.scenario_path = scenario_paths.get(&result.id).cloned();
        }
    }
    Ok(())
}

pub(super) fn compare_search_rows(
    left: &SearchRow,
    right: &SearchRow,
    exact_terms: &HashSet<String>,
) -> Ordering {
    let left_exact = exact_terms.contains(&left.result.id.to_lowercase());
    let right_exact = exact_terms.contains(&right.result.id.to_lowercase());
    match (left_exact, right_exact) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => left
            .score
            .partial_cmp(&right.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.result.kind.cmp(&right.result.kind))
            .then_with(|| left.result.id.cmp(&right.result.id)),
    }
}

pub(super) fn fts_query(terms: &[String]) -> String {
    terms
        .iter()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn exact_terms(terms: &[String]) -> HashSet<String> {
    let mut exact = terms
        .iter()
        .map(|term| term.to_lowercase())
        .collect::<HashSet<_>>();
    exact.insert(terms.join(" ").to_lowercase());
    exact
}

pub(super) fn decode_tags(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn has_all_tags(tags: &[String], filters: &[&str]) -> bool {
    filters
        .iter()
        .all(|filter| tags.iter().any(|tag| tag == filter))
}

pub(super) fn clean_snippet(snippet: &str) -> String {
    snippet.split_whitespace().collect::<Vec<_>>().join(" ")
}
