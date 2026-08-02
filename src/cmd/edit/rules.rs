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
pub enum NestedScalarMode {
    String,
    NonEmptyString,
    Semver,
    Enum {
        allowed: &'static [&'static str],
        invalid_msg: &'static str,
        code: Option<DiagnosticCode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedObjectSetMode {
    AcceptanceCriterion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedListValueCodec {
    RequirementBinding,
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
    pub value_codec: Option<NestedListValueCodec>,
    pub set_mode: Option<NestedScalarMode>,
    pub object_set_mode: Option<NestedObjectSetMode>,
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
pub struct NestedStatusListSpec {
    pub status_key: &'static str,
    pub text_key: &'static str,
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

pub fn root_field_names(artifact: &str) -> Vec<&'static str> {
    let mut names = SIMPLE_RULES
        .iter()
        .filter(|rule| rule.artifact == artifact)
        .map(|rule| rule.name)
        .chain(
            NESTED_RULES
                .iter()
                .filter(|rule| rule.artifact == artifact)
                .map(|rule| rule.root),
        )
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    names
}

pub fn nested_child_names(node: &NestedNodeRule) -> Vec<&'static str> {
    let mut names = node
        .fields
        .iter()
        .map(|field| field.name)
        .collect::<Vec<_>>();
    names.sort_unstable();
    names
}

pub fn simple_field_supports_verb(artifact: &str, field: &str, verb: Verb) -> bool {
    simple_field_rule(artifact, field).is_some_and(|rule| rule.verbs.contains(&verb.as_str()))
}

#[cfg(test)]
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

pub fn nested_status_list_spec(node: &NestedNodeRule) -> Option<NestedStatusListSpec> {
    if node.kind != NestedNodeKind::List {
        return None;
    }
    let item = node.item?;
    if item.kind != NestedNodeKind::Object {
        return None;
    }
    let text_key = node.text_key?;
    let status_key = item
        .fields
        .iter()
        .find(|field| field.name == "status" && field.node.kind == NestedNodeKind::Scalar)?
        .name;
    Some(NestedStatusListSpec {
        status_key,
        text_key,
    })
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
    fn test_nested_rule_lookup() -> Result<(), Box<dyn std::error::Error>> {
        let rule = nested_root_rule("adr", "alternatives").ok_or("rule should exist")?;
        assert_eq!(rule.node.kind, NestedNodeKind::List);
        assert_eq!(EDIT_RULES_VERSION, 3);
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
    fn test_rfc_current_changelog_rule_lookup() -> Result<(), Box<dyn std::error::Error>> {
        let rule = nested_root_rule("rfc", "changelog").ok_or("rule should exist")?;
        assert_eq!(rule.node.kind, NestedNodeKind::Object);
        assert!(rule.node.verbs.contains(&"get"));
        assert!(nested_field_supports_verb(
            "rfc",
            "changelog",
            "summary",
            Verb::Set
        ));
        assert!(!nested_field_supports_verb(
            "rfc",
            "changelog",
            "summary",
            Verb::Get
        ));
        Ok(())
    }

    #[test]
    fn test_nested_status_list_spec() -> Result<(), Box<dyn std::error::Error>> {
        let rule = nested_root_rule("adr", "alternatives").ok_or("rule should exist")?;
        let spec = nested_status_list_spec(rule.node).ok_or("status list should exist")?;
        assert_eq!(spec.status_key, "status");
        assert_eq!(spec.text_key, "text");

        let child = nested_field_rule("adr", "alternatives", "pros").ok_or("child exists")?;
        assert_eq!(nested_status_list_spec(child.node), None);
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
    fn test_conformance_requirement_rule_uses_structured_value_codec()
    -> Result<(), Box<dyn std::error::Error>> {
        let rule = nested_root_rule("conformance", "requirements").ok_or("rule should exist")?;
        assert_eq!(
            rule.node.value_codec,
            Some(NestedListValueCodec::RequirementBinding)
        );
        assert!(rule.node.verbs.contains(&"add"));
        assert!(rule.node.verbs.contains(&"remove"));
        Ok(())
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
