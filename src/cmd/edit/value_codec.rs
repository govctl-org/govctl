use super::matching::{MatchOptions, MatchUse, resolve_match_indices};
use super::rules::NestedListValueCodec;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::RequirementBinding;
use serde_json::{Map, Value};

pub(crate) fn parse_requirement_binding(value: &str) -> DiagnosticResult<RequirementBinding> {
    let (clause_ref, version) = value.rsplit_once('@').ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            "Requirement must use CLAUSE-ID@VERSION",
            value,
        )
    })?;
    if !clause_ref.starts_with("RFC-") || !clause_ref.contains(":C-") {
        return Err(Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            "Requirement must use a fully qualified RFC Clause ID",
            value,
        ));
    }
    semver::Version::parse(version).map_err(|_| {
        Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            format!("Invalid requirement version: {version}"),
            value,
        )
    })?;
    Ok(RequirementBinding {
        clause_ref: clause_ref.to_string(),
        version: version.to_string(),
    })
}

pub(super) fn decode(codec: NestedListValueCodec, value: &str) -> DiagnosticResult<Value> {
    match codec {
        NestedListValueCodec::RequirementBinding => {
            let binding = parse_requirement_binding(value)?;
            let mut object = Map::new();
            object.insert("ref".to_string(), Value::String(binding.clause_ref));
            object.insert("version".to_string(), Value::String(binding.version));
            Ok(Value::Object(object))
        }
    }
}

pub(super) fn encode(
    codec: NestedListValueCodec,
    value: &Value,
    id: &str,
) -> DiagnosticResult<String> {
    match codec {
        NestedListValueCodec::RequirementBinding => {
            let object = value.as_object().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    "Expected requirement binding object",
                    id,
                )
            })?;
            let clause_ref = object.get("ref").and_then(Value::as_str).ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    "Requirement binding is missing string field 'ref'",
                    id,
                )
            })?;
            let version = object
                .get("version")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::E0817PathTypeMismatch,
                        "Requirement binding is missing string field 'version'",
                        id,
                    )
                })?;
            Ok(format!("{clause_ref}@{version}"))
        }
    }
}

pub(super) fn resolve_remove_indices(
    codec: Option<NestedListValueCodec>,
    id: &str,
    field: &str,
    items: &[&str],
    options: &MatchOptions<'_>,
) -> DiagnosticResult<Vec<usize>> {
    let Some(codec) = codec else {
        return resolve_match_indices(id, field, items, options, MatchUse::Remove);
    };
    match codec {
        NestedListValueCodec::RequirementBinding => {
            if options.regex || options.all {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0802ConflictingArgs,
                    "Requirement removal supports only an exact value or indexed path",
                    id,
                ));
            }
            let value = options.pattern.ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0801MissingRequiredArg,
                    "Exact requirement removal requires a value",
                    id,
                )
            })?;
            let canonical = encode(codec, &decode(codec, value)?, id)?;
            let exact = MatchOptions {
                pattern: Some(canonical.as_str()),
                exact: true,
                ..MatchOptions::default()
            };
            resolve_match_indices(id, field, items, &exact, MatchUse::Remove)
        }
    }
}

pub(super) fn validate_remaining(
    codec: Option<NestedListValueCodec>,
    remaining: usize,
    id: &str,
) -> DiagnosticResult<()> {
    if codec == Some(NestedListValueCodec::RequirementBinding) && remaining == 0 {
        return Err(Diagnostic::new(
            DiagnosticCode::E1305ConformanceGraphInvalid,
            "A Conformance Case must retain at least one requirement",
            id,
        ));
    }
    Ok(())
}
