use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::ChangelogCategory;
use regex::Regex;

#[derive(Debug, Clone, Default)]
pub struct MatchOptions<'a> {
    pub pattern: Option<&'a str>,
    pub at: Option<i32>,
    pub exact: bool,
    pub regex: bool,
    pub all: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MatchOptionsOwned {
    pub pattern: Option<String>,
    pub at: Option<i32>,
    pub exact: bool,
    pub regex: bool,
    pub all: bool,
}

impl MatchOptionsOwned {
    pub fn as_match_options(&self) -> MatchOptions<'_> {
        MatchOptions {
            pattern: self.pattern.as_deref(),
            at: self.at,
            exact: self.exact,
            regex: self.regex,
            all: self.all,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MatchUse {
    Remove,
    TickSingle,
}

fn strip_category_prefix(pattern: &str) -> &str {
    ChangelogCategory::strip_rendered_prefix(pattern).unwrap_or(pattern)
}

pub(super) fn resolve_match_indices(
    id: &str,
    field: &str,
    items: &[&str],
    opts: &MatchOptions,
    use_case: MatchUse,
) -> DiagnosticResult<Vec<usize>> {
    if items.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0812FieldEmpty,
            format!("Field {}.{} is empty", id, field),
            id,
        ));
    }

    if opts.pattern.is_none() && opts.at.is_none() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            format!(
                "Remove from {}.{} requires a pattern or --at <index>",
                id, field
            ),
            id,
        ));
    }

    let indices: Vec<usize> = if let Some(idx) = opts.at {
        let len = items.len() as i32;
        let actual_idx = if idx < 0 { len + idx } else { idx };
        if actual_idx < 0 || actual_idx >= len {
            return Err(Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!(
                    "Index {} out of range (array has {} items)",
                    idx,
                    items.len()
                ),
                "array",
            ));
        }
        vec![actual_idx as usize]
    } else {
        let raw_pattern = opts.pattern.unwrap_or("<index>");
        let pattern = if opts.regex {
            raw_pattern
        } else {
            strip_category_prefix(raw_pattern)
        };
        let matches = if opts.regex {
            let re = Regex::new(pattern).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0806InvalidPattern,
                    format!("Invalid regex: {}", e),
                    id,
                )
            })?;
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| re.is_match(s))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        } else if opts.exact {
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == pattern)
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        } else {
            let pattern_lower = pattern.to_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| s.to_lowercase().contains(&pattern_lower))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };

        if matches.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!("No items match '{}' in {}.{}", raw_pattern, id, field),
                id,
            ));
        }
        matches
    };

    if indices.len() == 1 || (use_case == MatchUse::Remove && opts.all) {
        return Ok(indices);
    }

    let pattern = opts.pattern.unwrap_or("");
    let hint = if use_case == MatchUse::Remove {
        "Options:\n  • Use more specific pattern\n  • Use --at <index> to select one\n  • Use --all to remove all matches"
    } else {
        "Use more specific pattern or --at <index> to select one"
    };
    let mut msg = format!(
        "{} items match '{}' in {}.{}:\n",
        indices.len(),
        pattern,
        id,
        field
    );
    for &i in &indices {
        msg.push_str(&format!("  [{}] {}\n", i, items[i]));
    }
    msg.push('\n');
    msg.push_str(hint);
    Err(Diagnostic::new(
        DiagnosticCode::E0807AmbiguousMatch,
        msg,
        id,
    ))
}

#[cfg(test)]
mod tests;
