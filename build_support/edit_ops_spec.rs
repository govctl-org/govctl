use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub(super) struct EditOpsSpec {
    pub(super) version: u32,
    pub(super) aliases: Vec<AliasRule>,
    pub(super) legacy_prefixes: Vec<LegacyPrefixRule>,
    pub(super) simple_rules: Vec<SimpleFieldRule>,
    pub(super) runtime_fields: Vec<RuntimeFieldRule>,
    pub(super) nested_rules: Vec<NestedRootRule>,
    pub(super) validation_rules: Vec<FieldValidationRule>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AliasRule {
    pub(super) alias: String,
    pub(super) canonical: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct LegacyPrefixRule {
    pub(super) prefix: String,
    pub(super) allowed_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct NestedRootRule {
    pub(super) artifact: String,
    pub(super) root: String,
    pub(super) content_path: Vec<String>,
    pub(super) node: NestedNodeRule,
}

#[derive(Debug, Deserialize)]
pub(super) struct NestedFieldRule {
    pub(super) name: String,
    pub(super) node: NestedNodeRule,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum NestedNodeRule {
    Scalar {
        verbs: Vec<String>,
        set_mode: Option<RuntimeSetMode>,
    },
    Object {
        verbs: Vec<String>,
        fields: Vec<NestedFieldRule>,
    },
    List {
        verbs: Vec<String>,
        text_key: Option<String>,
        item: Box<NestedNodeRule>,
    },
}

#[derive(Debug, Deserialize)]
pub(super) struct SimpleFieldRule {
    pub(super) artifact: String,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) verbs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RuntimeFieldRule {
    pub(super) artifact: String,
    pub(super) name: String,
    pub(super) get: Option<RuntimeGetRule>,
    pub(super) set: Option<RuntimeSetRule>,
    pub(super) list_path: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RuntimeGetRule {
    pub(super) path: Vec<String>,
    pub(super) render: String,
    pub(super) status_key: Option<String>,
    pub(super) text_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RuntimeSetRule {
    pub(super) path: Vec<String>,
    pub(super) mode: RuntimeSetMode,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum RuntimeSetMode {
    String,
    Integer,
    Enum {
        allowed: Vec<String>,
        invalid_msg: String,
        code: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub(super) struct FieldValidationRule {
    pub(super) artifact: String,
    pub(super) field: String,
    pub(super) rule: String,
}

pub(super) fn load_edit_ops_spec(
    spec_path: &Path,
    schema_path: &Path,
) -> Result<EditOpsSpec, Box<dyn Error>> {
    let spec_text = fs::read_to_string(spec_path)?;
    let schema_text = fs::read_to_string(schema_path)?;
    let spec_value: Value = serde_json::from_str(&spec_text)?;
    let schema_value: Value = serde_json::from_str(&schema_text)?;

    validate_spec_against_schema(&schema_value, &spec_value)?;
    let spec: EditOpsSpec = serde_json::from_value(spec_value)?;
    validate_runtime_fields(&spec)?;
    Ok(spec)
}

fn validate_spec_against_schema(schema: &Value, instance: &Value) -> Result<(), Box<dyn Error>> {
    let compiled = jsonschema::validator_for(schema)
        .map_err(|err| format!("invalid edit-ops schema: {err}"))?;
    let mut diagnostics: Vec<String> = compiled
        .iter_errors(instance)
        .map(|err| err.to_string())
        .collect();
    if !diagnostics.is_empty() {
        diagnostics.sort();
        diagnostics.dedup();
        let body = diagnostics
            .into_iter()
            .map(|d| format!("  - {d}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("edit-ops.json failed schema validation:\n{body}").into());
    }
    Ok(())
}

fn validate_runtime_fields(spec: &EditOpsSpec) -> Result<(), Box<dyn Error>> {
    let mut seen = HashSet::new();
    for field in &spec.runtime_fields {
        let key = format!("{}:{}", field.artifact, field.name);
        if !seen.insert(key.clone()) {
            return Err(format!("duplicate runtime_fields entry in SSOT: {key}").into());
        }
        let simple = spec
            .simple_rules
            .iter()
            .find(|rule| rule.artifact == field.artifact && rule.name == field.name)
            .ok_or_else(|| format!("runtime_fields entry missing matching simple_rule: {}", key))?;

        if field.get.is_some() && !simple.verbs.iter().any(|v| v == "get") {
            return Err(format!(
                "runtime_fields {} defines get but simple_rules does not allow get",
                key
            )
            .into());
        }
        if field.set.is_some() && !simple.verbs.iter().any(|v| v == "set") {
            return Err(format!(
                "runtime_fields {} defines set but simple_rules does not allow set",
                key
            )
            .into());
        }
        if field.list_path.is_some() && !simple.verbs.iter().any(|v| v == "add" || v == "remove") {
            return Err(format!(
                "runtime_fields {} defines list_path but simple_rules has no add/remove verb",
                key
            )
            .into());
        }
    }
    Ok(())
}
