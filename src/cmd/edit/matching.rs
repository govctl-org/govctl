use crate::diagnostic::{Diagnostic, DiagnosticCode};
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
) -> anyhow::Result<Vec<usize>> {
    if items.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0812FieldEmpty,
            format!("Field {}.{} is empty", id, field),
            id,
        )
        .into());
    }

    if opts.pattern.is_none() && opts.at.is_none() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            format!(
                "Remove from {}.{} requires a pattern or --at <index>",
                id, field
            ),
            id,
        )
        .into());
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
            )
            .into());
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
            )
            .into());
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
    Err(Diagnostic::new(DiagnosticCode::E0807AmbiguousMatch, msg, id).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_case_insensitive_substring_match() -> Result<(), Box<dyn std::error::Error>> {
        let items = ["first item", "Second Item", "third"];
        let opts = MatchOptions {
            pattern: Some("second"),
            ..Default::default()
        };

        let indices = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::TickSingle,
        )?;

        assert_eq!(indices, vec![1]);
        Ok(())
    }

    #[test]
    fn strips_known_category_prefixes_before_matching() -> Result<(), Box<dyn std::error::Error>> {
        let items = ["some text", "other text"];
        let opts = MatchOptions {
            pattern: Some("fixed: some text"),
            ..Default::default()
        };

        let indices = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::TickSingle,
        )?;

        assert_eq!(indices, vec![0]);
        Ok(())
    }

    #[test]
    fn does_not_strip_non_rendered_category_aliases() {
        let items = ["some text"];
        let opts = MatchOptions {
            pattern: Some("fix: some text"),
            ..Default::default()
        };

        let result = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::Remove,
        );

        assert!(
            result.is_err(),
            "fix: is a parser alias, not a rendered prefix"
        );
    }

    #[test]
    fn allows_remove_all_for_multiple_matches() -> Result<(), Box<dyn std::error::Error>> {
        let items = ["apple", "apricot", "banana"];
        let opts = MatchOptions {
            pattern: Some("ap"),
            all: true,
            ..Default::default()
        };

        let indices = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::Remove,
        )?;

        assert_eq!(indices, vec![0, 1]);
        Ok(())
    }

    #[test]
    fn resolves_negative_indices_from_end() -> Result<(), Box<dyn std::error::Error>> {
        let items = ["first", "second", "third"];
        let opts = MatchOptions {
            at: Some(-1),
            ..Default::default()
        };

        let indices = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::TickSingle,
        )?;

        assert_eq!(indices, vec![2]);
        Ok(())
    }

    #[test]
    fn rejects_ambiguous_tick_matches_without_all() -> Result<(), Box<dyn std::error::Error>> {
        let items = ["test-one", "test-two", "other"];
        let opts = MatchOptions {
            pattern: Some("test"),
            ..Default::default()
        };

        let Err(err) = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::TickSingle,
        ) else {
            return Err("ambiguous tick match should fail".into());
        };

        assert!(err.to_string().contains("2 items match"));
        Ok(())
    }

    #[test]
    fn rejects_invalid_regex_patterns() -> Result<(), Box<dyn std::error::Error>> {
        let items = ["a", "b"];
        let opts = MatchOptions {
            pattern: Some("[invalid"),
            regex: true,
            ..Default::default()
        };

        let Err(err) = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::TickSingle,
        ) else {
            return Err("invalid regex should fail".into());
        };

        assert!(err.to_string().contains("Invalid regex"));
        Ok(())
    }

    #[test]
    fn rejects_no_match_patterns() -> Result<(), Box<dyn std::error::Error>> {
        let items = ["apple", "banana", "cherry"];
        let opts = MatchOptions {
            pattern: Some("xyz"),
            ..Default::default()
        };

        let Err(err) = resolve_match_indices(
            "WI-1",
            "acceptance_criteria",
            &items,
            &opts,
            MatchUse::Remove,
        ) else {
            return Err("missing match should fail".into());
        };

        assert!(err.to_string().contains("No items match"));
        Ok(())
    }
}
