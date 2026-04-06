//! Path-based nested field addressing per [[ADR-0029]].
//!
//! Parses field paths like `alt[0].pros[1]` into structured segments
//! for nested access into ADR alternatives, work item journal entries, etc.

use super::rules as edit_rules;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use winnow::Parser;
use winnow::ascii::digit1;
use winnow::combinator::{delimited, eof, opt, separated, terminated};
use winnow::error::{ContextError, ErrMode};
use winnow::token::{any, take_while};

type ParseErr = ErrMode<ContextError>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawPathSegment {
    name: String,
    index: Option<String>,
}

/// A single segment in a field path (e.g., `alt[0]` → name="alternatives", index=Some(0)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    pub name: String,
    pub index: Option<i32>,
}

/// A parsed field path with one or more segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPath {
    pub segments: Vec<PathSegment>,
}

impl FieldPath {
    /// True if this is a single segment with no index (flat field like "title").
    pub fn is_simple(&self) -> bool {
        self.segments.len() == 1 && self.segments[0].index.is_none()
    }

    /// Get the flat field name if this is a simple path.
    pub fn as_simple(&self) -> Option<&str> {
        if self.is_simple() {
            Some(&self.segments[0].name)
        } else {
            None
        }
    }

    /// True if the last segment has an explicit index.
    pub fn has_terminal_index(&self) -> bool {
        self.segments.last().is_some_and(|s| s.index.is_some())
    }

    /// Normalize aliases on each path segment (`alt` -> `alternatives`, etc.).
    pub fn normalize_aliases(mut self) -> Self {
        for seg in &mut self.segments {
            seg.name = normalize_segment_name(&seg.name);
        }
        self
    }

    /// Collapse legacy prefixes into their canonical field-path form.
    ///
    /// `content.decision` → `decision`, `govctl.status` → `status`, etc.
    pub fn collapse_legacy_prefixes(mut self) -> Self {
        if self.segments.len() >= 2 && self.segments[0].index.is_none() {
            let prefix = self.segments[0].name.as_str();
            let field = self.segments[1].name.as_str();
            if edit_rules::can_collapse_legacy_prefix(prefix, field) {
                self.segments.remove(0);
            }
        }
        self
    }
}

impl std::fmt::Display for FieldPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            f.write_str(&seg.name)?;
            if let Some(idx) = seg.index {
                write!(f, "[{idx}]")?;
            }
        }
        Ok(())
    }
}

/// Normalize a single field name, expanding aliases to canonical form.
fn normalize_segment_name(name: &str) -> String {
    edit_rules::normalize_alias(name).to_string()
}

/// Parse a field path string into a `FieldPath`.
///
/// Grammar: `segment ('.' segment | '[' index ']')*`
/// where `segment` is `[a-z_][a-z0-9_]*` and `index` is `-?[0-9]+`.
#[allow(dead_code)]
pub fn parse_field_path(input: &str) -> anyhow::Result<FieldPath> {
    parse_raw_field_path(input).map(FieldPath::normalize_aliases)
}

/// Parse a field path string into raw segments, without alias normalization.
pub fn parse_raw_field_path(input: &str) -> anyhow::Result<FieldPath> {
    if input.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0814InvalidPath,
            "Field path cannot be empty",
            "path",
        )
        .into());
    }

    let raw_segments = terminated(path_segments_parser, eof)
        .parse(input)
        .map_err(|_| {
            Diagnostic::new(
                DiagnosticCode::E0814InvalidPath,
                format!("Invalid field path: {input}"),
                "path",
            )
        })?;

    let mut segments = Vec::with_capacity(raw_segments.len());
    for raw in raw_segments {
        let index = match raw.index {
            Some(text) => Some(text.parse::<i32>().map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0814InvalidPath,
                    format!("Invalid field path: invalid index '{text}'"),
                    "path",
                )
            })?),
            None => None,
        };
        segments.push(PathSegment {
            name: raw.name,
            index,
        });
    }

    Ok(FieldPath { segments })
}

fn path_segments_parser(input: &mut &str) -> Result<Vec<RawPathSegment>, ParseErr> {
    separated(1.., parse_segment_raw, '.').parse_next(input)
}

fn parse_segment_raw(input: &mut &str) -> Result<RawPathSegment, ParseErr> {
    let name = parse_name_raw(input)?;
    let index = opt(parse_index_text).parse_next(input)?;
    Ok(RawPathSegment { name, index })
}

fn parse_name_raw(rest: &mut &str) -> Result<String, ParseErr> {
    let first = any.verify(|c: &char| is_name_start(*c)).parse_next(rest)?;
    let suffix: &str = take_while(0.., is_name_char).parse_next(rest)?;
    Ok(format!("{first}{suffix}"))
}

fn parse_index_text(rest: &mut &str) -> Result<String, ParseErr> {
    let (sign, digits): (Option<char>, &str) =
        delimited('[', (opt('-'), digit1), ']').parse_next(rest)?;

    let mut idx = String::new();
    if sign.is_some() {
        idx.push('-');
    }
    idx.push_str(digits);
    Ok(idx)
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_lowercase() || c == '_'
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
}

/// Resolve an index (0-based, negative from end) against an array length.
pub fn resolve_index(idx: i32, len: usize) -> anyhow::Result<usize> {
    let len_i = len as i32;
    let actual = if idx < 0 { len_i + idx } else { idx };
    if actual < 0 || actual >= len_i {
        return Err(Diagnostic::new(
            DiagnosticCode::E0816PathIndexOutOfBounds,
            format!("Index {idx} out of range (array has {len} items)"),
            "path",
        )
        .into());
    }
    Ok(actual as usize)
}

/// Require that a segment has an index, resolve it against a length.
pub fn require_index(seg: &PathSegment, len: usize) -> anyhow::Result<usize> {
    let idx = seg.index.ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0816PathIndexOutOfBounds,
            format!(
                "Field '{}' requires an index (e.g., {}[0])",
                seg.name, seg.name
            ),
            "path",
        )
    })?;
    resolve_index(idx, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // parse_field_path — happy paths
    // =========================================================================

    #[test]
    fn test_simple_field() {
        let p = parse_field_path("title").unwrap();
        assert_eq!(p.segments.len(), 1);
        assert_eq!(p.segments[0].name, "title");
        assert_eq!(p.segments[0].index, None);
        assert!(p.is_simple());
        assert_eq!(p.as_simple(), Some("title"));
    }

    #[test]
    fn test_indexed_field() {
        let p = parse_field_path("alternatives[0]").unwrap();
        assert_eq!(p.segments.len(), 1);
        assert_eq!(p.segments[0].name, "alternatives");
        assert_eq!(p.segments[0].index, Some(0));
        assert!(!p.is_simple());
        assert!(p.has_terminal_index());
    }

    #[test]
    fn test_dotted_path() {
        let p = parse_field_path("alt[0].pros").unwrap();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[0].name, "alternatives");
        assert_eq!(p.segments[0].index, Some(0));
        assert_eq!(p.segments[1].name, "pros");
        assert_eq!(p.segments[1].index, None);
        assert!(!p.is_simple());
        assert!(!p.has_terminal_index());
    }

    #[test]
    fn test_dotted_path_with_terminal_index() {
        let p = parse_field_path("alt[0].pros[1]").unwrap();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[0].name, "alternatives");
        assert_eq!(p.segments[0].index, Some(0));
        assert_eq!(p.segments[1].name, "pros");
        assert_eq!(p.segments[1].index, Some(1));
        assert!(p.has_terminal_index());
    }

    #[test]
    fn test_negative_index() {
        let p = parse_field_path("alt[-1]").unwrap();
        assert_eq!(p.segments[0].index, Some(-1));
    }

    // =========================================================================
    // Alias expansion
    // =========================================================================

    #[test]
    fn test_alias_alt() {
        let p = parse_field_path("alt[0]").unwrap();
        assert_eq!(p.segments[0].name, "alternatives");
    }

    #[test]
    fn test_raw_parse_keeps_alias_token() {
        let p = parse_raw_field_path("alt[0]").unwrap();
        assert_eq!(p.segments[0].name, "alt");
    }

    #[test]
    fn test_alias_ac() {
        let p = parse_field_path("ac[0]").unwrap();
        assert_eq!(p.segments[0].name, "acceptance_criteria");
    }

    #[test]
    fn test_alias_pro_con() {
        let p = parse_field_path("alt[0].pro[0]").unwrap();
        assert_eq!(p.segments[1].name, "pros");
        let p = parse_field_path("alt[0].con[0]").unwrap();
        assert_eq!(p.segments[1].name, "cons");
    }

    #[test]
    fn test_alias_reason() {
        let p = parse_field_path("alt[0].reason").unwrap();
        assert_eq!(p.segments[1].name, "rejection_reason");
    }

    #[test]
    fn test_alias_desc() {
        let p = parse_field_path("desc").unwrap();
        assert_eq!(p.segments[0].name, "description");
    }

    // =========================================================================
    // Legacy prefix collapse
    // =========================================================================

    #[test]
    fn test_collapse_content_decision() {
        let p = parse_field_path("content.decision")
            .unwrap()
            .collapse_legacy_prefixes();
        assert!(p.is_simple());
        assert_eq!(p.as_simple(), Some("decision"));
    }

    #[test]
    fn test_collapse_govctl_status() {
        let p = parse_field_path("govctl.status")
            .unwrap()
            .collapse_legacy_prefixes();
        assert!(p.is_simple());
        assert_eq!(p.as_simple(), Some("status"));
    }

    #[test]
    fn test_no_collapse_when_indexed() {
        // content[0].decision should NOT collapse — content has index
        let p = parse_field_path("content[0].decision")
            .unwrap()
            .collapse_legacy_prefixes();
        assert_eq!(p.segments.len(), 2);
    }

    #[test]
    fn test_no_collapse_non_legacy_prefix() {
        let p = parse_field_path("alt[0].pros")
            .unwrap()
            .collapse_legacy_prefixes();
        assert_eq!(p.segments.len(), 2);
    }

    #[test]
    fn test_no_collapse_unknown_legacy_field() {
        let p = parse_field_path("content.unknown")
            .unwrap()
            .collapse_legacy_prefixes();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[0].name, "content");
    }

    #[test]
    fn test_collapse_legacy_prefix_for_deeper_path() {
        let p = parse_field_path("content.alternatives[0].pros")
            .unwrap()
            .collapse_legacy_prefixes();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.segments[0].name, "alternatives");
        assert_eq!(p.segments[0].index, Some(0));
        assert_eq!(p.segments[1].name, "pros");
    }

    // =========================================================================
    // Error cases
    // =========================================================================

    #[test]
    fn test_empty_path() {
        assert!(parse_field_path("").is_err());
    }

    #[test]
    fn test_invalid_start_char() {
        assert!(parse_field_path("0invalid").is_err());
        assert!(parse_field_path("[0]").is_err());
        assert!(parse_field_path("Alt").is_err());
    }

    #[test]
    fn test_double_index() {
        assert!(parse_field_path("alt[0][1]").is_err());
    }

    #[test]
    fn test_full_consumption_rejects_trailing_garbage() {
        assert!(parse_field_path("alt[0]oops").is_err());
    }

    #[test]
    fn test_empty_index() {
        assert!(parse_field_path("alt[]").is_err());
    }

    #[test]
    fn test_unclosed_bracket() {
        assert!(parse_field_path("alt[0").is_err());
    }

    #[test]
    fn test_trailing_dot() {
        assert!(parse_field_path("alt.").is_err());
    }

    // =========================================================================
    // resolve_index
    // =========================================================================

    #[test]
    fn test_resolve_index_zero() {
        assert_eq!(resolve_index(0, 3).unwrap(), 0);
    }

    #[test]
    fn test_resolve_index_positive() {
        assert_eq!(resolve_index(2, 5).unwrap(), 2);
    }

    #[test]
    fn test_resolve_index_negative() {
        assert_eq!(resolve_index(-1, 3).unwrap(), 2);
        assert_eq!(resolve_index(-3, 3).unwrap(), 0);
    }

    #[test]
    fn test_resolve_index_out_of_bounds() {
        assert!(resolve_index(3, 3).is_err());
        assert!(resolve_index(-4, 3).is_err());
    }

    #[test]
    fn test_resolve_index_empty_array() {
        assert!(resolve_index(0, 0).is_err());
    }

    // =========================================================================
    // require_index
    // =========================================================================

    #[test]
    fn test_require_index_present() {
        let seg = PathSegment {
            name: "alt".to_string(),
            index: Some(1),
        };
        assert_eq!(require_index(&seg, 3).unwrap(), 1);
    }

    #[test]
    fn test_require_index_missing() {
        let seg = PathSegment {
            name: "alt".to_string(),
            index: None,
        };
        assert!(require_index(&seg, 3).is_err());
    }
}
