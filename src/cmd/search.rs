mod document;
mod index;
mod query;

use crate::artifact_catalog::CatalogKind;
use crate::cmd::output::{command_table, print_json_array};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::{ListTarget, OutputFormat};
use comfy_table::{Attribute, Cell};
use serde::Serialize;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_path: Option<String>,
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
    let connection = index::open(config)?;
    index::sync(config, &connection, &kinds, reindex)?;
    query::query_index(
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
        .flat_map(|argument| argument.split_whitespace())
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
        return CatalogKind::ALL.to_vec();
    }

    let mut kinds = Vec::new();
    for filter in type_filters {
        let kind = match filter {
            ListTarget::Rfc => CatalogKind::Rfc,
            ListTarget::Clause => CatalogKind::Clause,
            ListTarget::Adr => CatalogKind::Adr,
            ListTarget::Work => CatalogKind::Work,
            ListTarget::Guard => CatalogKind::Guard,
            ListTarget::Conformance => CatalogKind::Conformance,
        };
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    kinds
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

#[cfg(test)]
mod tests {
    use super::index::encode_tags;
    use super::query::{
        SearchRow, clean_snippet, compare_search_rows, decode_tags, exact_terms, fts_query,
        has_all_tags,
    };
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn query_terms_rejects_empty_input_and_splits_whitespace()
    -> Result<(), Box<dyn std::error::Error>> {
        let empty = match query_terms(&[]) {
            Ok(_) => return Err(std::io::Error::other("empty search returned Ok").into()),
            Err(error) => error,
        };
        assert_eq!(empty.code, DiagnosticCode::E0801MissingRequiredArg);

        let terms = query_terms(&["  cache  index ".to_string(), "ADR-0001".to_string()])?;
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

    #[test]
    fn equal_relevance_orders_by_kind_then_id() {
        let row = |kind: &str, id: &str| SearchRow {
            result: SearchResult {
                kind: kind.to_string(),
                id: id.to_string(),
                title: String::new(),
                path: String::new(),
                snippet: String::new(),
                score: Some(1.0),
                status: None,
                scenario_path: None,
            },
            tags: vec![],
            score: 1.0,
        };
        let mut rows = [
            row("work", "WI-2026-01-01-001"),
            row("adr", "ADR-0002"),
            row("adr", "ADR-0001"),
        ];
        rows.sort_by(|left, right| compare_search_rows(left, right, &HashSet::new()));
        assert_eq!(
            rows.iter()
                .map(|entry| (entry.result.kind.as_str(), entry.result.id.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("adr", "ADR-0001"),
                ("adr", "ADR-0002"),
                ("work", "WI-2026-01-01-001")
            ]
        );
    }
}
