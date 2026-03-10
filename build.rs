use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct EditOpsSpec {
    version: u32,
    aliases: Vec<AliasRule>,
    legacy_prefixes: Vec<LegacyPrefixRule>,
    simple_rules: Vec<SimpleFieldRule>,
    runtime_fields: Vec<RuntimeFieldRule>,
    nested_rules: Vec<NestedRootRule>,
    validation_rules: Vec<FieldValidationRule>,
}

#[derive(Debug, Deserialize)]
struct AliasRule {
    alias: String,
    canonical: String,
}

#[derive(Debug, Deserialize)]
struct LegacyPrefixRule {
    prefix: String,
    allowed_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct NestedRootRule {
    artifact: String,
    root: String,
    content_path: Vec<String>,
    text_key: Option<String>,
    requires_index: bool,
    max_depth: usize,
    fields: Vec<NestedFieldRule>,
}

#[derive(Debug, Deserialize)]
struct NestedFieldRule {
    name: String,
    kind: String,
    verbs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SimpleFieldRule {
    artifact: String,
    name: String,
    kind: String,
    verbs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeFieldRule {
    artifact: String,
    name: String,
    get: Option<RuntimeGetRule>,
    set: Option<RuntimeSetRule>,
    list_path: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RuntimeGetRule {
    path: Vec<String>,
    render: String,
    status_key: Option<String>,
    text_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeSetRule {
    path: Vec<String>,
    mode: RuntimeSetMode,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RuntimeSetMode {
    String,
    OptionalString {
        empty_as_null: bool,
    },
    Enum {
        allowed: Vec<String>,
        invalid_msg: String,
        code: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct FieldValidationRule {
    artifact: String,
    field: String,
    rule: String,
}

fn main() {
    // Recompile if any embedded .claude/ assets change
    // Skills
    println!("cargo:rerun-if-changed=.claude/skills/discuss/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/gov/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/quick/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/rfc-writer/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/adr-writer/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/wi-writer/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/commit/SKILL.md");
    println!("cargo:rerun-if-changed=.claude/skills/migrate/SKILL.md");
    // Agents
    println!("cargo:rerun-if-changed=.claude/agents/rfc-reviewer.md");
    println!("cargo:rerun-if-changed=.claude/agents/adr-reviewer.md");
    println!("cargo:rerun-if-changed=.claude/agents/wi-reviewer.md");
    println!("cargo:rerun-if-changed=.claude/agents/compliance-checker.md");

    // Edit rules SSOT + schema (ADR-0030)
    println!("cargo:rerun-if-changed=gov/schema/edit-ops.schema.json");
    println!("cargo:rerun-if-changed=gov/schema/edit-ops.json");

    generate_edit_rules().expect("failed to generate edit rules from SSOT");
}

fn generate_edit_rules() -> Result<(), Box<dyn Error>> {
    let spec_path = Path::new("gov/schema/edit-ops.json");
    let schema_path = Path::new("gov/schema/edit-ops.schema.json");
    let spec_text = fs::read_to_string(spec_path)?;
    let schema_text = fs::read_to_string(schema_path)?;
    let spec_value: Value = serde_json::from_str(&spec_text)?;
    let schema_value: Value = serde_json::from_str(&schema_text)?;

    validate_spec_against_schema(&schema_value, &spec_value)?;
    let spec: EditOpsSpec = serde_json::from_value(spec_value)?;
    validate_runtime_fields(&spec)?;
    let rendered = render_edit_rules(&spec)?;
    let rendered_runtime = render_edit_runtime(&spec)?;
    let out_dir = std::env::var("OUT_DIR")?;
    let out_path = Path::new(&out_dir).join("edit_rules_generated.rs");
    let out_runtime_path = Path::new(&out_dir).join("edit_runtime_generated.rs");
    fs::write(out_path, rendered)?;
    fs::write(out_runtime_path, rendered_runtime)?;
    Ok(())
}

fn validate_spec_against_schema(schema: &Value, instance: &Value) -> Result<(), Box<dyn Error>> {
    let compiled =
        jsonschema::validator_for(schema).map_err(|err| format!("invalid edit-ops schema: {err}"))?;
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

fn render_edit_rules(spec: &EditOpsSpec) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();
    out.push_str("// @generated by build.rs from gov/schema/edit-ops.json\n");
    out.push_str("// Do not edit manually.\n\n");
    out.push_str(&format!(
        "#[allow(dead_code)]\npub const EDIT_RULES_VERSION: u32 = {};\n\n",
        spec.version
    ));

    out.push_str("define_alias_resolver! {\n");
    for alias in &spec.aliases {
        out.push_str(&format!(
            "    ({:?}, {:?}),\n",
            alias.alias, alias.canonical
        ));
    }
    out.push_str("}\n\n");

    out.push_str("define_legacy_prefix_resolver! {\n");
    for rule in &spec.legacy_prefixes {
        out.push_str(&format!("    ({:?}, [", rule.prefix));
        for (idx, field) in rule.allowed_fields.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("{field:?}"));
        }
        out.push_str("]),\n");
    }
    out.push_str("}\n\n");

    out.push_str("pub const SIMPLE_RULES: &[SimpleFieldRule] = &[\n");
    for field in &spec.simple_rules {
        let kind = match field.kind.as_str() {
            "scalar" => "Scalar",
            "list" => "List",
            other => return Err(format!("unknown simple field kind in SSOT: {other}").into()),
        };
        out.push_str("    SimpleFieldRule {\n");
        out.push_str(&format!("        artifact: {:?},\n", field.artifact));
        out.push_str(&format!("        name: {:?},\n", field.name));
        out.push_str(&format!("        kind: FieldKind::{kind},\n"));
        out.push_str("        verbs: &[\n");
        for verb in &field.verbs {
            out.push_str(&format!("            {verb:?},\n"));
        }
        out.push_str("        ],\n");
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");

    out.push_str("pub const NESTED_RULES: &[NestedRootRule] = &[\n");
    for root in &spec.nested_rules {
        out.push_str("    NestedRootRule {\n");
        out.push_str(&format!("        artifact: {:?},\n", root.artifact));
        out.push_str(&format!("        root: {:?},\n", root.root));
        out.push_str(&format!(
            "        content_path: {},\n",
            runtime_path_expr(&root.content_path)
        ));
        out.push_str(&format!(
            "        text_key: {},\n",
            match &root.text_key {
                Some(k) => format!("Some({k:?})"),
                None => "None".to_string(),
            }
        ));
        out.push_str(&format!(
            "        requires_index: {},\n",
            root.requires_index
        ));
        out.push_str(&format!("        max_depth: {},\n", root.max_depth));
        out.push_str("        fields: &[\n");
        for field in &root.fields {
            let kind = match field.kind.as_str() {
                "scalar" => "Scalar",
                "list" => "List",
                other => return Err(format!("unknown field kind in SSOT: {other}").into()),
            };

            out.push_str("            NestedFieldRule {\n");
            out.push_str(&format!("                name: {:?},\n", field.name));
            out.push_str(&format!("                kind: FieldKind::{kind},\n"));
            out.push_str("                verbs: &[\n");
            for verb in &field.verbs {
                out.push_str(&format!("                    {verb:?},\n"));
            }
            out.push_str("                ],\n");
            out.push_str("            },\n");
        }
        out.push_str("        ],\n");
        out.push_str("    },\n");
    }
    out.push_str("];\n");

    out.push('\n');
    out.push_str("pub const VALIDATION_RULES: &[FieldValidationRule] = &[\n");
    for rule in &spec.validation_rules {
        let kind = match rule.rule.as_str() {
            "semver" => "Semver",
            "clause_superseded_by" => "ClauseSupersededBy",
            "artifact_ref" => "ArtifactRef",
            "enum_value" => "EnumValue",
            other => return Err(format!("unknown validation rule in SSOT: {other}").into()),
        };
        out.push_str("    FieldValidationRule {\n");
        out.push_str(&format!("        artifact: {:?},\n", rule.artifact));
        out.push_str(&format!("        field: {:?},\n", rule.field));
        out.push_str(&format!("        kind: ValidationKind::{kind},\n"));
        out.push_str("    },\n");
    }
    out.push_str("];\n");

    Ok(out)
}

fn render_edit_runtime(spec: &EditOpsSpec) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();
    out.push_str("// @generated by build.rs from gov/schema/edit-ops.json\n");
    out.push_str("// Do not edit manually.\n\n");
    out.push_str("const RUNTIME_FIELDS: &[RuntimeFieldEntry] = &[\n");
    for field in &spec.runtime_fields {
        out.push_str("    RuntimeFieldEntry {\n");
        out.push_str(&format!(
            "        artifact: {},\n",
            runtime_artifact_expr(&field.artifact)?
        ));
        out.push_str(&format!("        field: {:?},\n", field.name));
        out.push_str(&format!(
            "        get: {},\n",
            runtime_get_expr(field.get.as_ref())?
        ));
        out.push_str(&format!(
            "        set: {},\n",
            runtime_set_expr(field.set.as_ref())?
        ));
        out.push_str(&format!(
            "        list_path: {},\n",
            runtime_list_path_expr(field.list_path.as_ref())
        ));
        out.push_str("    },\n");
    }
    out.push_str("];\n");
    Ok(out)
}

fn runtime_artifact_expr(artifact: &str) -> Result<&'static str, Box<dyn Error>> {
    match artifact {
        "rfc" => Ok("ArtifactType::Rfc"),
        "clause" => Ok("ArtifactType::Clause"),
        "adr" => Ok("ArtifactType::Adr"),
        "work" => Ok("ArtifactType::WorkItem"),
        other => Err(format!("unknown runtime artifact in SSOT: {other}").into()),
    }
}

fn runtime_path_expr(path: &[String]) -> String {
    let mut out = String::from("&[");
    for (idx, seg) in path.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{seg:?}"));
    }
    out.push(']');
    out
}

fn runtime_list_path_expr(path: Option<&Vec<String>>) -> String {
    match path {
        Some(path) => format!("Some({})", runtime_path_expr(path)),
        None => "None".to_string(),
    }
}

fn runtime_get_expr(get: Option<&RuntimeGetRule>) -> Result<String, Box<dyn Error>> {
    let Some(get) = get else {
        return Ok("None".to_string());
    };
    let render = match get.render.as_str() {
        "scalar" => "RenderMode::Scalar".to_string(),
        "csv_strings" => "RenderMode::CsvStrings".to_string(),
        "line_strings" => "RenderMode::LineStrings".to_string(),
        "status_lines" => {
            let status_key = get.status_key.as_ref().ok_or_else(|| {
                "runtime get render=status_lines requires status_key in SSOT".to_string()
            })?;
            let text_key = get.text_key.as_ref().ok_or_else(|| {
                "runtime get render=status_lines requires text_key in SSOT".to_string()
            })?;
            format!(
                "RenderMode::StatusLines {{ status_key: {:?}, text_key: {:?} }}",
                status_key, text_key
            )
        }
        other => return Err(format!("unknown runtime get render in SSOT: {other}").into()),
    };
    Ok(format!(
        "Some(SimpleFieldSpec {{ path: {}, render: {} }})",
        runtime_path_expr(&get.path),
        render
    ))
}

fn runtime_set_expr(set: Option<&RuntimeSetRule>) -> Result<String, Box<dyn Error>> {
    let Some(set) = set else {
        return Ok("None".to_string());
    };
    let mode = match &set.mode {
        RuntimeSetMode::String => "SetMode::String".to_string(),
        RuntimeSetMode::OptionalString { empty_as_null } => format!(
            "SetMode::OptionalString {{ empty_as_null: {} }}",
            empty_as_null
        ),
        RuntimeSetMode::Enum {
            allowed,
            invalid_msg,
            code,
        } => {
            let allowed_expr = runtime_path_expr(allowed);
            let code_expr = match code {
                Some(code) => format!("Some(DiagnosticCode::{code})"),
                None => "None".to_string(),
            };
            format!(
                "SetMode::Enum {{ allowed: {}, invalid_msg: {:?}, code: {} }}",
                allowed_expr, invalid_msg, code_expr
            )
        }
    };
    Ok(format!(
        "Some(SimpleSetSpec {{ path: {}, mode: {} }})",
        runtime_path_expr(&set.path),
        mode
    ))
}
