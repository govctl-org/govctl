//! Edit path rules generated from JSON SSOT (ADR-0030).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Scalar,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedFieldRule {
    pub name: &'static str,
    pub kind: FieldKind,
    pub verbs: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedRootRule {
    pub artifact: &'static str,
    pub root: &'static str,
    pub content_path: &'static [&'static str],
    pub requires_index: bool,
    pub max_depth: usize,
    pub fields: &'static [NestedFieldRule],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimpleFieldRule {
    pub artifact: &'static str,
    pub name: &'static str,
    pub kind: FieldKind,
    pub verbs: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationKind {
    Semver,
    ClauseSupersededBy,
    ArtifactRef,
    EnumValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldValidationRule {
    pub artifact: &'static str,
    pub field: &'static str,
    pub kind: ValidationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Get,
    Set,
    Add,
    Remove,
    Tick,
}

impl Verb {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Set => "set",
            Self::Add => "add",
            Self::Remove => "remove",
            Self::Tick => "tick",
        }
    }
}

macro_rules! define_alias_resolver {
    ($(($alias:literal, $canonical:literal)),* $(,)?) => {
        pub fn normalize_alias(name: &str) -> &str {
            match name {
                $($alias => $canonical,)*
                _ => name,
            }
        }
    };
}

macro_rules! define_legacy_prefix_resolver {
    ($(($prefix:literal, [$($field:literal),* $(,)?])),* $(,)?) => {
        pub fn can_collapse_legacy_prefix(prefix: &str, field: &str) -> bool {
            match prefix {
                $($prefix => matches!(field, $($field)|*),)*
                _ => false,
            }
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/edit_rules_generated.rs"));

pub fn nested_root_rule(artifact: &str, root: &str) -> Option<&'static NestedRootRule> {
    NESTED_RULES
        .iter()
        .find(|rule| rule.artifact == artifact && rule.root == root)
}

pub fn simple_field_rule(artifact: &str, field: &str) -> Option<&'static SimpleFieldRule> {
    SIMPLE_RULES
        .iter()
        .find(|rule| rule.artifact == artifact && rule.name == field)
}

pub fn simple_field_supports_verb(artifact: &str, field: &str, verb: Verb) -> bool {
    simple_field_rule(artifact, field).is_some_and(|rule| rule.verbs.contains(&verb.as_str()))
}

pub fn nested_field_rule(
    artifact: &str,
    root: &str,
    field: &str,
) -> Option<&'static NestedFieldRule> {
    nested_root_rule(artifact, root).and_then(|rule| rule.fields.iter().find(|f| f.name == field))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn nested_field_supports_verb(artifact: &str, root: &str, field: &str, verb: Verb) -> bool {
    nested_field_rule(artifact, root, field).is_some_and(|rule| rule.verbs.contains(&verb.as_str()))
}

pub fn field_validation_rule(artifact: &str, field: &str) -> Option<&'static FieldValidationRule> {
    VALIDATION_RULES
        .iter()
        .find(|rule| rule.artifact == artifact && rule.field == field)
}

pub fn field_validation_kind(artifact: &str, field: &str) -> Option<ValidationKind> {
    field_validation_rule(artifact, field).map(|rule| rule.kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aliases_generated() {
        assert_eq!(normalize_alias("alt"), "alternatives");
        assert_eq!(normalize_alias("reason"), "rejection_reason");
        assert_eq!(normalize_alias("unknown"), "unknown");
    }

    #[test]
    fn test_legacy_prefix_generation() {
        assert!(can_collapse_legacy_prefix("content", "decision"));
        assert!(!can_collapse_legacy_prefix("content", "nonexistent"));
        assert!(!can_collapse_legacy_prefix("nope", "decision"));
    }

    #[test]
    fn test_nested_rule_lookup() {
        let rule = nested_root_rule("adr", "alternatives").expect("rule should exist");
        assert_eq!(rule.max_depth, 2);
        assert!(rule.requires_index);
        assert_eq!(EDIT_RULES_VERSION, 1);
    }

    #[test]
    fn test_nested_field_supports_verb() {
        assert!(nested_field_supports_verb(
            "adr",
            "alternatives",
            "pros",
            Verb::Add
        ));
        assert!(!nested_field_supports_verb(
            "adr",
            "alternatives",
            "status",
            Verb::Add
        ));
    }

    #[test]
    fn test_simple_field_supports_verb() {
        assert!(simple_field_supports_verb(
            "adr",
            "alternatives",
            Verb::Tick
        ));
        assert!(!simple_field_supports_verb(
            "adr",
            "superseded_by",
            Verb::Set
        ));
    }

    #[test]
    fn test_validation_rule_lookup() {
        assert_eq!(
            field_validation_kind("rfc", "version"),
            Some(ValidationKind::Semver)
        );
        assert_eq!(
            field_validation_kind("clause", "superseded_by"),
            Some(ValidationKind::ClauseSupersededBy)
        );
        assert_eq!(field_validation_kind("rfc", "owners"), None);
    }
}
