//! Edit path rules generated from JSON SSOT (ADR-0030).

use crate::diagnostic::DiagnosticCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Scalar,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedNodeKind {
    Scalar,
    Object,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum NestedScalarMode {
    String,
    OptionalString {
        empty_as_null: bool,
    },
    Integer,
    Enum {
        allowed: &'static [&'static str],
        invalid_msg: &'static str,
        code: Option<DiagnosticCode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedChildRule {
    pub name: &'static str,
    pub node: &'static NestedNodeRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedNodeRule {
    pub kind: NestedNodeKind,
    pub verbs: &'static [&'static str],
    pub text_key: Option<&'static str>,
    pub set_mode: Option<NestedScalarMode>,
    pub item: Option<&'static NestedNodeRule>,
    pub fields: &'static [NestedChildRule],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedRootRule {
    pub artifact: &'static str,
    pub root: &'static str,
    pub content_path: &'static [&'static str],
    pub node: &'static NestedNodeRule,
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
) -> Option<&'static NestedChildRule> {
    let rule = nested_root_rule(artifact, root)?;
    match rule.node.kind {
        NestedNodeKind::Object => rule.node.fields.iter().find(|f| f.name == field),
        NestedNodeKind::List => {
            let item = rule.node.item?;
            if item.kind != NestedNodeKind::Object {
                return None;
            }
            item.fields.iter().find(|f| f.name == field)
        }
        NestedNodeKind::Scalar => None,
    }
}

#[cfg(test)]
pub fn nested_field_supports_verb(artifact: &str, root: &str, field: &str, verb: Verb) -> bool {
    nested_field_rule(artifact, root, field)
        .is_some_and(|rule| rule.node.verbs.contains(&verb.as_str()))
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
    fn test_nested_rule_lookup() -> Result<(), Box<dyn std::error::Error>> {
        let rule = nested_root_rule("adr", "alternatives").ok_or("rule should exist")?;
        assert_eq!(rule.node.kind, NestedNodeKind::List);
        assert_eq!(EDIT_RULES_VERSION, 2);
        Ok(())
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
    fn test_nested_object_root_lookup() -> Result<(), Box<dyn std::error::Error>> {
        let rule = nested_root_rule("guard", "check").ok_or("rule should exist")?;
        assert_eq!(rule.node.kind, NestedNodeKind::Object);
        let child = nested_field_rule("guard", "check", "timeout_secs").ok_or("child exists")?;
        assert_eq!(child.node.kind, NestedNodeKind::Scalar);
        Ok(())
    }

    #[test]
    fn test_simple_field_supports_verb() {
        assert!(simple_field_supports_verb("adr", "alternatives", Verb::Add));
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
